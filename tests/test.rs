use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use tempfile::TempDir;

struct TestBuilder {
    root: TempDir,
    sources: Vec<PathBuf>,
}

impl TestBuilder {
    /// Initialize a new tmp directory
    pub fn new() -> Self {
        Self {
            root: TempDir::new().expect("cannot create temporary directory"),
            sources: Vec::new(),
        }
    }

    #[allow(unused)]
    pub fn add_file<Q: AsRef<Path>, P: AsRef<Path>>(&mut self, src: P, dest: Q) {
        let dest = self.root.path().join(dest);
        let src_display = src.as_ref().display();
        let dest_display: &Path = dest.as_ref();
        fs::copy(&src, &dest).expect(&format!(
            "unable to copy {} to {}",
            src_display,
            dest_display.display()
        ));
        self.sources.push(dest);
    }

    pub fn add_src<P: AsRef<Path>>(&mut self, path: P, code: &str) {
        let dest = self.root.path().join(path);
        fs::write(&dest, code).expect(&format!("unable to write code to path: {}", dest.display()));
        self.sources.push(dest);
    }

    fn get_class_names(&self) -> Vec<String> {
        // TODO: check case for inner classes
        self.sources
            .iter()
            .filter_map(|p| {
                let filename = p.to_str().unwrap();
                if filename.ends_with(".java") {
                    Some(filename.trim_end_matches(".java").to_owned() + ".class")
                } else {
                    None
                }
            })
            .collect()
    }

    fn compile(&self) -> PathBuf {
        let _javac = Command::new("javac")
            .args(&self.sources)
            .current_dir(self.root.path())
            .status()
            .expect("javac failed");
        let classes = self.get_class_names();
        assert!(classes.len() > 0);
        let _d8 = Command::new("d8")
            .args(&classes)
            .args(&["--output", &self.root.path().display().to_string()])
            .current_dir(self.root.path())
            .status()
            .expect(&format!("'d8 {:?}' failed", &classes));
        self.root.path().join("classes.dex")
    }
}

macro_rules! assert_has_access_flags {
    ($item: ident, [ $($flag: ident),+ ], $msg:expr) => {
        $(
            assert!($item.access_flags().contains(AccessFlags::$flag), $msg);
        )*
    };

    ($item: ident, [ $($flag: ident),+ ]) => {
        assert_has_access_flags!($item,  [$($flag),+], "")
    }
}

// TODO: support test attributes if necessary
macro_rules! test {
    ($test_name: ident, $({ $fname:expr => $code:expr });+,$test_func:expr) => {
        #[test]
        fn $test_name() {
            use dex::DexReader;
            let mut builder = TestBuilder::new();
            $(
               builder.add_src($fname, $code);
            )*
            let dex_path = builder.compile();
            let dex = DexReader::from_file(dex_path.as_path());
            assert!(dex.is_ok());
            $test_func(dex.unwrap());
        }
    };

    ($test_name: ident, $({ $fname:expr => $code:expr }),+) => {
        test!($test_name, $({$fname => $code},)+ |_| {});
    }
}

test!(
    test_dex_from_file_works,
    {
        "Main.java" =>
        r#"
            class Main {
             public static void main(String[] args) {
                System.out.println("1 + 1 = " + 1 + 1);
             }
            }
       "#
    }
);

test!(
    test_find_class_by_name,
    {
        "Main.java" => r#"
            class Main {}
        "#
    };
    {
        "Day.java" => r#"
            public enum Day {
               SUNDAY, MONDAY, TUESDAY, WEDNESDAY,
               THURSDAY, FRIDAY, SATURDAY 
            }
        "#
    };
    {
        "SuperClass.java" => r#"
            class SuperClass {}
        "#
    };
    {
        "MyInterface.java" => r#"
            interface MyInterface {
                String interfaceMethod(int x, String y);
            }
        "#
    },
    |dex: dex::Dex<_>| {
        use dex::class::AccessFlags;
        assert_eq!(dex.header().class_defs_size(), 4);
        let find = |name| {
            let class = dex.find_class_by_name(name);
            assert!(class.is_ok());
            let class = class.unwrap();
            assert!(class.is_some());
            class.unwrap()
        };
        let interface = find("LMyInterface;");
        assert!(interface.access_flags().contains(AccessFlags::INTERFACE));

        let enum_class = find("LDay;");
        assert!(enum_class.access_flags().contains(AccessFlags::ENUM));
    }
);

test!(
    test_class_exists,
    {
        "Main.java" =>
        r#"
            class Main {}
        "#
    },
    |dex: dex::Dex<_>| {
        let class = dex.find_class_by_name("LMain;");
        assert!(class.is_ok());
        let class = class.unwrap();
        assert!(class.is_some());
    }
);

// TODO: add tests for interface fields, initial values, annotations on fields
test!(
    test_fields,
    {
        "Main.java" => r#"
          class Main<T, K extends Main> {
              public static int staticVar = 42;
              final double finalVar = 32.0d;
              private String privateField;
              public String publicField;
              protected String protectedField;
              int[] arrayField;
              Day enumField;
              T genericField;
              K genericField2;
          }
        "#
    };
    {
        "Day.java" => r#"
            public enum Day {
               SUNDAY, MONDAY, TUESDAY, WEDNESDAY,
               THURSDAY, FRIDAY, SATURDAY 
            }
        "#
    },
    |dex: dex::Dex<_>| {
        use dex::field::AccessFlags;
        let class = dex.find_class_by_name("LMain;").unwrap().unwrap();
        assert_eq!(class.static_fields().count(), 1);
        assert_eq!(class.instance_fields().count(), 8);
        let find = |name, jtype| {
            let field = class.fields().find(|f| f.name() == name);
            assert!(field.is_some(), format!("name: {}, type: {}", name, jtype));
            let field = field.unwrap();
            assert_eq!(field.jtype(), jtype);
            field
        };
        let static_field = find("staticVar", "I");
        assert_has_access_flags!(static_field, [STATIC, PUBLIC]);

        let final_field = find("finalVar", "D");
        assert_has_access_flags!(final_field, [FINAL]);

        let protected_field = find("protectedField", "Ljava/lang/String;");
        assert_has_access_flags!(protected_field, [PROTECTED]);

        let private_field = find("privateField", "Ljava/lang/String;");
        assert_has_access_flags!(private_field, [PRIVATE]);

        let public_field = find("publicField", "Ljava/lang/String;");
        assert_has_access_flags!(public_field, [PUBLIC]);

        let array_field = find("arrayField", "[I");
        assert!(array_field.access_flags().is_empty());

        let generic_field = find("genericField", "Ljava/lang/Object;");
        assert!(generic_field.access_flags().is_empty());

        let generic_field = find("genericField2", "LMain;");
        assert!(generic_field.access_flags().is_empty());



        // TODO: find out why d8 fails with warning:
        // d8 is from build-tools:29.0.2
        // Type `java.lang.Enum` was not found, it is required for default or static interface
        // methods desugaring of `Day Day.valueOf(java.lang.String)`
        // let enum_field = find("enumField");
        // assert!(enum_field.access_flags().contains(AccessFlags::ENUM));
        
    }
);

test!(
    test_field_values,
    {
        "FieldValues.java" => r#"
            import java.util.function.BiFunction;
            class FieldValues {
                private final static byte b = 120;
                private final static byte nb = -100;
                private final static short s = 10000;
                private final static short ns = -12048;
                private final static int i = 12540;
                private final static int ni = -12540;
                private final static long l = 43749374797L;
                private final static long nl = -43749374797L;
                private final static float f = 9.30f;
                private final static float nf = -9.34f;
                private final static double d = 4374.9437493749374d;
                private final static double nd = -1257.9374937493d;
                private final static boolean bo = true;
                private final static boolean nbo = false;
                private final static char c = 'm';
                private final static String nullString = null;
                private final static String nonNullString = "fjdljfdlj";
                private final static BiFunction<String, Integer, Integer> remapper = (k, v) -> v == null ? 42 : v + 41;
                private final static Runnable r = () -> { System.out.println("runnable"); };
                private final static int[] array = new int[]{1, 2, 3};
            }
        "#
    },
    |dex: dex::Dex<_>| {
        use dex::string::DexString;
        use dex::encoded_value::EncodedValue;
        let class = dex.find_class_by_name("LFieldValues;").expect("error getting FieldValues.class").expect("class not found");
        assert_eq!(class.fields().count(), 20);
        let get_value = |name| {
            let field = class.fields().find(|f| f.name() == name);
            assert!(field.is_some(), "field: {}", name);
            let field = field.unwrap();
            field.initial_value()
        };
        assert_eq!(*get_value("b"), Some(EncodedValue::Byte(120)));
        assert_eq!(*get_value("nb"), Some(EncodedValue::Byte(-100)));
        assert_eq!(*get_value("s"), Some(EncodedValue::Short(10000)));
        // TODO: fix sign-extended numbers
        //assert_eq!(*get_value("ns"), Some(EncodedValue::Short(-12048)));
        assert_eq!(*get_value("i"), Some(EncodedValue::Int(12540)));
        // TODO: fix sign-extended numbers
        // assert_eq!(*get_value("ni"), Some(EncodedValue::Int(-12540)));
        assert_eq!(*get_value("l"), Some(EncodedValue::Long(43749374797)));
        // TODO: fix sign-extended numbers
        // assert_eq!(*get_value("nl"), Some(EncodedValue::Long(-43749374797)));
        assert_eq!(*get_value("f"), Some(EncodedValue::Float(9.30)));
        assert_eq!(*get_value("nf"), Some(EncodedValue::Float(-9.34)));
        assert_eq!(*get_value("d"), Some(EncodedValue::Double(4374.943749374937)));
        assert_eq!(*get_value("nd"), Some(EncodedValue::Double(-1257.9374937493)));
        assert_eq!(*get_value("bo"), Some(EncodedValue::Boolean(true)));
        assert_eq!(*get_value("nbo"), Some(EncodedValue::Boolean(false)));
        assert_eq!(*get_value("c"), Some(EncodedValue::Char(b'm'.into())));
        assert_eq!(*get_value("nullString"), Some(EncodedValue::Null));
        assert_eq!(*get_value("nonNullString"), Some(EncodedValue::String(DexString::from("fjdljfdlj".to_string()))));
        assert_eq!(*get_value("remapper"), Some(EncodedValue::Null));
        assert_eq!(*get_value("r"), Some(EncodedValue::Null));
        assert_eq!(*get_value("array"), Some(EncodedValue::Null));
    }
);

test!(
    test_interface,
    {
        "MyInterface.java" => r#"
            interface MyInterface {
                int x = 115;
                String j = "MyString";
                int interfaceMethod(String x, char y);
                default int interfaceMethod3(String y) {
                    System.out.println(y);
                    return 0;
                }
            }
        "#
    };
    {
        "MyInterface2.java" => r#"
            public interface MyInterface2 extends MyInterface {
                int interfaceMethod2(String y);
                default int interfaceMethod(String x, char y) {
                    System.out.println(x + y);
                    return 0;
                }
            }
        "#
    };
    {
        "MyInterface3.java" => r#"
            interface MyInterface3 {
                static int interfaceMethod3() {
                    return 3;
                }
            }
        "#
    },
    |dex: dex::Dex<_>| {
        use dex::class::AccessFlags;
        use dex::class::Class;
        let validate_interface_methods = |interface: &Class| {
            interface.methods().for_each(|m| {
                use dex::method::AccessFlags;
                assert_has_access_flags!(m, [PUBLIC, ABSTRACT], format!("interface method: {} doesn't have all attributes", m.name()));
                assert!(m.code().is_none(), format!("interface method: {} shouldn't have code item", m.name()));
            });
        };
        let validate_interface_fields = |interface: &Class| {
            interface.fields().for_each(|f| {
                use dex::field::AccessFlags;
                assert_has_access_flags!(f, [PUBLIC, STATIC, FINAL], format!("interface field: {} doesn't have all attributes", f.name()));
            });
        };

        let interface = dex.find_class_by_name("LMyInterface;");
        assert!(interface.is_ok());
        let interface = interface.unwrap();
        assert!(interface.is_some());
        let interface = interface.unwrap();
        assert_has_access_flags!(interface, [INTERFACE]);
        assert_eq!(interface.fields().count(), 2);
        assert_eq!(interface.methods().count(), 2);
        validate_interface_fields(&interface);
        validate_interface_methods(&interface);

        let interface2 = dex.find_class_by_name("LMyInterface2;");
        assert!(interface2.is_ok());
        let interface2 = interface2.unwrap();
        assert!(interface2.is_some());
        let interface2 = interface2.unwrap();
        assert_has_access_flags!(interface2, [PUBLIC, INTERFACE]);
        assert_eq!(interface2.methods().count(), 2);
        assert_eq!(interface2.fields().count(), 0);
        validate_interface_fields(&interface2);
        validate_interface_methods(&interface2);
        assert_eq!(interface2.interfaces(), &[interface.jtype().clone()]);

        // static methods in an interface are moved to a generated class marked SYNTHETIC.
        // HACK: name can be anything. but the current d8 generates a name that starts
        // with the original interface's name.
        let interface3 = dex.classes().find(|c| {
            let c = c.as_ref().expect("error finding synthetic class");
            c.jtype().type_descriptor().starts_with("LMyInterface3") &&
            c.access_flags().contains(AccessFlags::SYNTHETIC)
        });
        assert!(interface3.is_some());
        let interface3 = interface3.unwrap();
        assert!(interface3.is_ok());
        let interface3 = interface3.unwrap();
        assert_eq!(interface3.methods().count(), 1);
        let method = interface3.methods().take(1).next();
        assert!(method.is_some());
        let method = method.unwrap();
        {
            use dex::method::AccessFlags;
            assert_has_access_flags!(method, [STATIC]);
        }
    }
);

test!(
    test_abstract_classes,
    {
        "AbstractClass.java" => r#"
            abstract class AbstractClass {
                public static void staticMethod() {}
                public final void finalMethod() {}
                public abstract int abstractMethod();
            }
        "#
    },
    |dex: dex::Dex<_>| {
        use dex::class::AccessFlags;
        let abstract_class = dex.find_class_by_name("LAbstractClass;");
        assert!(abstract_class.is_ok());
        let abstract_class = abstract_class.unwrap();
        assert!(abstract_class.is_some());
        let abstract_class = abstract_class.unwrap();
        assert_has_access_flags!(abstract_class, [ABSTRACT]);
        assert_eq!(abstract_class.methods().count(), 4);
        let mut methods = abstract_class.methods().map(|m| m.name()).collect::<Vec<_>>();
        methods.sort();
        let expected = &mut ["<init>", "staticMethod", "abstractMethod", "finalMethod"];
        expected.sort();
        assert_eq!(&methods, expected);
        let abstract_method = abstract_class.methods().find(|m| m.name() == "abstractMethod").unwrap();
        {
            use dex::method::AccessFlags;
            assert!(abstract_method.code().is_none());
            assert_has_access_flags!(abstract_method, [PUBLIC, ABSTRACT]);
        }
    }
);

test!(
    test_enums,
    {
        "EnumClass.java" => r#"
            public enum EnumClass {
               SUNDAY, MONDAY, TUESDAY, WEDNESDAY,
               THURSDAY, FRIDAY, SATURDAY
            }
        "#
    },
    |dex: dex::Dex<_>| {
        use dex::class::AccessFlags;
        let enum_class = dex.find_class_by_name("LEnumClass;");
        assert!(enum_class.is_ok());
        let enum_class = enum_class.unwrap();
        assert!(enum_class.is_some());
        let enum_class = enum_class.unwrap();
        assert_has_access_flags!(enum_class, [ENUM]);
    }
);

// TODO: test method annotations
test!(
    test_methods,
    {
        "Main.java" => r#"
            import java.util.List;
            abstract class Main extends SuperClass implements MyInterface {
              // constructor 
              Main() {}

              // attributes
              void defaultMethod() {}
              final void finalMethod() {}
              static void staticMethod() {}
              public void publicMethod() {}
              private void privateMethod() {}
              protected void protectedMethod() {}

              // return values
              int primitiveReturnMethod() { return 0; }
              String classReturnMethod() { return null; }
              long[] arrayReturnMethod() { return new long[10]; }
              String[] objectArrayReturnMethod() { return new String[10]; }
              Day enumReturnMethod() { return Day.SUNDAY; }

              // params
              int primitiveParams(char u, short v, byte w, int x, long y, boolean z, double a, float b) { return 0; }
              String classParams(String x, String y) { return "22"; }
              void enumParam(Day day) {}
              void interfaceParam(MyInterface instance) {}
              void primitiveArrayParam(long[] instance) {}
              void objectArrayParam(String[] instance) {}
              private <T> void genericParamsMethod1(List<T> myList, int k) {}
              private <T> void genericParamsMethod2(T typeParam, int k) {}
              private void genericParamsMethod3(List<? super Main> typeParam, int k) {}
              private void genericParamsMethod4(List<? extends Main> typeParam, int k) {}
              private <T extends SuperClass> void genericParamWithExtendsClauseMethod(T typeParam) {}
              private <T extends SuperClass & MyInterface> void genericParamWithMultipleExtendsClauseMethod(T typeParam) {}
              public int varargsMethod(String... args) { return 1; }

              // overriden method
              @Override int superMethod(String y) { return 2; }
              
              // interface method
              public String interfaceMethod(int x, String y) { return y + x; }

              // native method
              public native String nativeMethod(int x, String y);

              // abstract method
              abstract int abstractMethod(int x);

              // synchronized method
              synchronized int synchronizedMethod(int y) { return 1; }
            }
        "#
    };
    {
        "Day.java" => r#"
            public enum Day {
               SUNDAY, MONDAY, TUESDAY, WEDNESDAY,
               THURSDAY, FRIDAY, SATURDAY 
            }
        "#
    };
    {
        "SuperClass.java" => r#"
            class SuperClass {
                int superMethod(String x) { return 1; }
                final int superMethod2(String x) { return 1; }
            }
        "#
    };
    {
        "MyInterface.java" => r#"
            interface MyInterface {
                String interfaceMethod(int x, String y);
            }
        "#
    },
    |dex: dex::Dex<_>| {
        use dex::method::AccessFlags;
        let class = dex.find_class_by_name("LMain;").unwrap().unwrap();
        assert_eq!(class.direct_methods().count(), 9);
        assert_eq!(class.virtual_methods().count(), 21);

        let find = |name, params: &[&str], return_type: &str| {
            let method = class.methods().find(|m| {
                m.name() == name && 
                    m.params().iter().map(|s| s.type_descriptor()).eq(params.iter()) &&
                    m.return_type() == return_type
            });
            assert!(method.is_some(), format!("method: {}, params: {:?}, return_type: {}", name, params, return_type));
            let method = method.unwrap();
            method
        };

        let default_method = find("defaultMethod", &[], "V");
        assert!(default_method.code().is_some());
        assert!(default_method.access_flags().is_empty());
        assert_eq!(default_method.shorty(), "V");

        let final_method = find("finalMethod", &[], "V");
        assert!(final_method.code().is_some());
        assert_has_access_flags!(final_method, [FINAL]);
        assert_eq!(final_method.shorty(), "V");

        let static_method = find("staticMethod", &[], "V");
        assert!(static_method.code().is_some());
        assert_has_access_flags!(static_method, [STATIC]);
        assert_eq!(static_method.shorty(), "V");

        let public_method = find("publicMethod", &[], "V");
        assert!(public_method.code().is_some());
        assert_has_access_flags!(public_method, [PUBLIC]);
        assert_eq!(public_method.shorty(), "V");

        let private_method = find("privateMethod", &[], "V");
        assert!(private_method.code().is_some());
        assert_has_access_flags!(private_method, [PRIVATE]);
        assert_eq!(private_method.shorty(), "V");

        let protected_method = find("protectedMethod", &[], "V");
        assert!(protected_method.code().is_some());
        assert_has_access_flags!(protected_method, [PROTECTED]);
        assert_eq!(protected_method.shorty(), "V");


        let primitive_return_method = find("primitiveReturnMethod", &[], "I");
        assert!(primitive_return_method.code().is_some());
        assert!(primitive_return_method.access_flags().is_empty());
        assert_eq!(primitive_return_method.shorty(), "I");

        let class_return_method = find("classReturnMethod", &[], "Ljava/lang/String;");
        assert!(primitive_return_method.code().is_some());
        assert!(class_return_method.access_flags().is_empty());
        assert_eq!(class_return_method.shorty(), "L");

        let array_return_method = find("arrayReturnMethod", &[], "[J");
        assert!(array_return_method.code().is_some());
        assert!(array_return_method.access_flags().is_empty());
        assert_eq!(array_return_method.shorty(), "L");

        let object_array_return_method = find("objectArrayReturnMethod", &[], "[Ljava/lang/String;");
        assert!(object_array_return_method.code().is_some());
        assert!(array_return_method.access_flags().is_empty());
        assert!(object_array_return_method.access_flags().is_empty());
        assert_eq!(object_array_return_method.shorty(), "L");

        let enum_return_method = find("enumReturnMethod", &[], "LDay;");
        assert!(enum_return_method.code().is_some());
        assert!(enum_return_method.access_flags().is_empty());
        assert_eq!(enum_return_method.shorty(), "L");


        let primitive_params_method = find("primitiveParams", &["C", "S", "B", "I", "J", "Z", "D", "F"], "I");
        assert!(primitive_params_method.code().is_some());
        assert!(primitive_params_method.access_flags().is_empty());
        assert_eq!(primitive_params_method.shorty(), "ICSBIJZDF");
        
        let class_params_method = find("classParams", &["Ljava/lang/String;", "Ljava/lang/String;"], "Ljava/lang/String;");
        assert!(class_params_method.code().is_some());
        assert!(class_params_method.access_flags().is_empty());
        assert_eq!(class_params_method.shorty(), "LLL");

        let enum_params_method = find("enumParam", &["LDay;"], "V");
        assert!(enum_params_method.code().is_some());
        assert!(enum_params_method.access_flags().is_empty());
        assert_eq!(enum_params_method.shorty(), "VL");
        
        let primitive_array_params_method = find("primitiveArrayParam", &["[J"], "V");
        assert!(primitive_array_params_method.code().is_some());
        assert!(primitive_array_params_method.access_flags().is_empty());
        assert_eq!(primitive_array_params_method.shorty(), "VL");

        let object_array_params_method = find("objectArrayParam", &["[Ljava/lang/String;"], "V");
        assert!(object_array_params_method.code().is_some());
        assert!(object_array_params_method.access_flags().is_empty());
        assert_eq!(object_array_params_method.shorty(), "VL");

        let interface_params_method = find("interfaceParam", &["LMyInterface;"], "V");
        assert!(interface_params_method.code().is_some());
        assert!(interface_params_method.access_flags().is_empty());
        assert_eq!(interface_params_method.shorty(), "VL");

        let generic_params_method  = find("genericParamsMethod1", &["Ljava/util/List;", "I"], "V");
        assert!(generic_params_method.code().is_some());
        assert_has_access_flags!(generic_params_method, [PRIVATE]);
        assert_eq!(generic_params_method.shorty(), "VLI");

        let generic_params_method  = find("genericParamsMethod2", &["Ljava/lang/Object;", "I"], "V");
        assert!(generic_params_method.code().is_some());
        assert_has_access_flags!(generic_params_method, [PRIVATE]);
        assert_eq!(generic_params_method.shorty(), "VLI");

        let generic_params_method  = find("genericParamsMethod3", &["Ljava/util/List;", "I"], "V");
        assert!(generic_params_method.code().is_some());
        assert_has_access_flags!(generic_params_method, [PRIVATE]);
        assert_eq!(generic_params_method.shorty(), "VLI");

        let generic_params_method  = find("genericParamsMethod4", &["Ljava/util/List;", "I"], "V");
        assert!(generic_params_method.code().is_some());
        assert_has_access_flags!(generic_params_method, [PRIVATE]);
        assert_eq!(generic_params_method.shorty(), "VLI");


        let generic_params_method  = find("genericParamWithExtendsClauseMethod", &["LSuperClass;"], "V");
        assert!(generic_params_method.code().is_some());
        assert_has_access_flags!(generic_params_method, [PRIVATE]);
        assert_eq!(generic_params_method.shorty(), "VL");

        let generic_params_method  = find("genericParamWithMultipleExtendsClauseMethod", &["LSuperClass;"], "V");
        assert!(generic_params_method.code().is_some());
        assert_has_access_flags!(generic_params_method, [PRIVATE]);
        assert_eq!(generic_params_method.shorty(), "VL");


        let varargs_method = find("varargsMethod", &["[Ljava/lang/String;"], "I");
        assert!(varargs_method.code().is_some());
        assert_has_access_flags!(varargs_method, [PUBLIC, VARARGS]);
        assert_eq!(varargs_method.shorty(), "IL");


        let super_method = find("superMethod", &["Ljava/lang/String;"], "I");
        assert!(super_method.code().is_some());
        assert!(super_method.access_flags().is_empty());
        assert_eq!(super_method.shorty(), "IL");

        let super_method2 = class.fields().find(|m| m.name() == "superMethod2");
        assert!(super_method2.is_none(), "super method 2 is not overriden, so it shouldn't be there");


        let interface_method = find("interfaceMethod", &["I", "Ljava/lang/String;"], "Ljava/lang/String;");
        assert!(interface_method.code().is_some());
        assert_has_access_flags!(interface_method, [PUBLIC]);
        assert_eq!(interface_method.shorty(), "LIL");


        let native_method = find("nativeMethod", &["I", "Ljava/lang/String;"], "Ljava/lang/String;");
        assert!(native_method.code().is_none());
        assert_has_access_flags!(native_method, [PUBLIC, NATIVE]);
        assert_eq!(native_method.shorty(), "LIL");

        let abstract_method = find("abstractMethod", &["I"], "I");
        assert!(abstract_method.code().is_none());
        assert_has_access_flags!(abstract_method, [ABSTRACT]);
        assert_eq!(abstract_method.shorty(), "II");

        let synchronized_method = find("synchronizedMethod", &["I"], "I");
        assert!(synchronized_method.code().is_some());
        assert_has_access_flags!(synchronized_method, [DECLARED_SYNCHRONIZED]);
        assert_eq!(synchronized_method.shorty(), "II");
    }
);
