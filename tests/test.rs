use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use tempfile::TempDir;
use dex::field::FieldId;
use dex::method::MethodId;

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
        let android_lib_path = env::var("ANDROID_LIB_PATH").expect("$ANDROID_LIB_PATH not set");
        let _javac = Command::new("javac")
            .args(&self.sources)
            .current_dir(self.root.path())
            .status()
            .expect("javac failed");
        let classes = self.get_class_names();
        assert!(classes.len() > 0);
        let _d8 = Command::new("d8")
            .args(&classes)
            .args(&["--lib", &android_lib_path])
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
            assert!($item.$flag(), $msg);
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
        assert_eq!(dex.header().class_defs_size(), 4);
        let find = |name| {
            let class = dex.find_class_by_name(name);
            assert!(class.is_ok());
            let class = class.unwrap();
            assert!(class.is_some());
            class.unwrap()
        };
        let interface = find("LMyInterface;");
        assert!(interface.is_interface());

        let enum_class = find("LDay;");
        assert!(enum_class.is_enum());
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

test!(
    test_annotations,
    {
        "Annotation.java" => r#"
            public @interface Annotation {
                public String name();
                public int value();
            }
        "#
    };
    {
        "RuntimeAnnotation.java" => r#"
            import java.lang.annotation.Retention;
            import java.lang.annotation.RetentionPolicy;
            @Retention(RetentionPolicy.RUNTIME)
            @interface RuntimeAnnotation {}
        "#
    };
    {
        "Main.java" => r#"
            @RuntimeAnnotation
            public class Main<T> {
                @RuntimeAnnotation @Annotation(name = "name", value = 5) private T field;
                @RuntimeAnnotation public void myMethod(@RuntimeAnnotation final int param1, T param2) {}
            }
        "#
    },
    |dex: dex::Dex<_>| {
        use dex::annotation::Visibility;
        use dex::encoded_value::EncodedValue;

        let annotation_class = dex.find_class_by_name("LAnnotation;");
        assert!(annotation_class.is_ok());
        let annotation_class = annotation_class.unwrap();
        assert!(annotation_class.is_some());
        let annotation_class = annotation_class.unwrap();
        assert_has_access_flags!(annotation_class, [is_public, is_annotation]);
        assert_eq!(annotation_class.methods().count(), 2);

        let runtime_annotation_class = dex.find_class_by_name("LRuntimeAnnotation;");
        assert!(runtime_annotation_class.is_ok());
        let runtime_annotation_class = runtime_annotation_class.unwrap();
        assert!(runtime_annotation_class.is_some());
        let runtime_annotation_class = runtime_annotation_class.unwrap();
        assert_has_access_flags!(runtime_annotation_class, [is_annotation]);


        let class = dex.find_class_by_name("LMain;");
        assert!(class.is_ok());
        let class = class.unwrap();
        assert!(class.is_some());
        let class = class.unwrap();
        let field = class.fields().find(|f| f.name() == "field");
        assert!(field.is_some());
        let field = field.unwrap();

        let annotation_item = field.annotations().iter().find(|i| i.jtype() == "LAnnotation;");
        assert!(annotation_item.is_some());
        let annotation_item = annotation_item.unwrap();
        assert_eq!(annotation_item.visibility(), Visibility::Build);
        let name = annotation_item.find_element("name");
        assert!(name.is_some());
        assert_eq!(name.unwrap().value(), "name");

        let value = annotation_item.find_element("value");
        assert!(value.is_some());
        assert_eq!(*value.unwrap().value(), EncodedValue::Int(5));


        let signature = field.signature();
        assert!(signature.is_ok());
        let signature = signature.unwrap();
        assert!(signature.is_some());
        let signature = signature.unwrap();
        assert_eq!(signature, "TT;".to_string());

        let method = class.methods().find(|n| n.name() == "myMethod");
        assert!(method.is_some());
        let method = method.unwrap();

        let signature = method.signature();
        assert!(signature.is_ok());
        let signature = signature.unwrap();
        assert!(signature.is_some());
        let signature = signature.unwrap();
        assert_eq!(signature, "(ITT;)V".to_string());

        let annotation_set_ref_list: Vec<_> = method.param_annotations().iter().collect();
        assert_eq!(annotation_set_ref_list.len(), 2);
        let first = &annotation_set_ref_list[0].annotations()[0];
        assert_eq!(first.jtype(), runtime_annotation_class.jtype());
        assert!(&annotation_set_ref_list[1].annotations().is_empty());

        let class_annotation = class.annotations().iter().find(|item| item.jtype() == "LRuntimeAnnotation;");
        assert!(class_annotation.is_some());

        let signature = class.signature();
        assert!(signature.is_ok());
        let signature = signature.unwrap();
        assert!(signature.is_some());
        let signature = signature.unwrap();
        assert_eq!(signature, "<T:Ljava/lang/Object;>Ljava/lang/Object;".to_string());

    }
);

test!(
    test_fields,
    {
        "Main.java" => r#"
          class Main<T, K extends Main> {
              boolean booleanVar;
              byte byteVar;
              short shortVar;
              char charVar;
              long longVar;
              float floatVar;
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
        let class = dex.find_class_by_name("LMain;").unwrap().unwrap();
        assert_eq!(class.static_fields().len(), 1);
        assert_eq!(class.instance_fields().len(), 14);
        let find = |name, jtype| {
            let field = class.fields().find(|f| f.name() == name);
            assert!(field.is_some(), "name: {}, type: {}", name, jtype);
            let field = field.unwrap();
            assert_eq!(field.jtype(), jtype);
            field
        };

        let find_id_item = |id: FieldId| {
            dex.field_ids().find(|f| if let Ok(f) = f {
                f.id() == id
            } else {
                false
            })
        };

        let boolean = find("booleanVar", "Z");
        assert!(boolean.jtype().is_bool());
        assert!(find_id_item(boolean.id()).is_some());

        let char_var = find("charVar", "C");
        assert!(char_var.jtype().is_char());
        assert!(find_id_item(char_var.id()).is_some());

        let short = find("shortVar", "S");
        assert!(short.jtype().is_short());
        assert!(find_id_item(short.id()).is_some());

        let float = find("floatVar", "F");
        assert!(float.jtype().is_float());
        assert!(find_id_item(float.id()).is_some());

        let long = find("longVar", "J");
        assert!(long.jtype().is_long());
        assert!(find_id_item(long.id()).is_some());

        let static_field = find("staticVar", "I");
        assert!(!static_field.jtype().is_reference());
        assert!(static_field.jtype().is_int());
        assert_has_access_flags!(static_field, [is_static, is_public]);
        assert!(find_id_item(static_field.id()).is_some());

        let final_field = find("finalVar", "D");
        assert!(!final_field.jtype().is_reference());
        assert!(final_field.jtype().is_double());
        assert_has_access_flags!(final_field, [is_final]);
        assert!(find_id_item(final_field.id()).is_some());

        let protected_field = find("protectedField", "Ljava/lang/String;");
        assert!(protected_field.jtype().is_reference());
        assert!(protected_field.jtype().is_class());
        assert_has_access_flags!(protected_field, [is_protected]);
        assert!(find_id_item(protected_field.id()).is_some());

        let private_field = find("privateField", "Ljava/lang/String;");
        assert_has_access_flags!(private_field, [is_private]);
        assert!(find_id_item(private_field.id()).is_some());

        let public_field = find("publicField", "Ljava/lang/String;");
        assert_has_access_flags!(public_field, [is_public]);
        assert!(find_id_item(public_field.id()).is_some());

        let array_field = find("arrayField", "[I");
        assert!(array_field.jtype().is_array());
        assert_eq!(array_field.jtype().array_dimensions(), Some(1));
        assert!(array_field.jtype().is_reference());
        assert!(array_field.access_flags().is_empty());
        assert!(find_id_item(array_field.id()).is_some());

        let generic_field = find("genericField", "Ljava/lang/Object;");
        assert!(generic_field.jtype().is_reference());
        assert_eq!(generic_field.jtype().array_dimensions(), None);
        assert!(generic_field.access_flags().is_empty());
        assert!(find_id_item(generic_field.id()).is_some());

        let generic_field = find("genericField2", "LMain;");
        assert!(generic_field.access_flags().is_empty());
        assert!(find_id_item(generic_field.id()).is_some());

        let enum_field = find("enumField", "LDay;");
        assert!(enum_field.access_flags().is_empty());
        assert!(find_id_item(enum_field.id()).is_some());
        
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
        assert_eq!(get_value("b"), Some(&EncodedValue::Byte(120)));
        assert_eq!(get_value("nb"), Some(&EncodedValue::Byte(-100)));
        assert_eq!(get_value("s"), Some(&EncodedValue::Short(10000)));
        assert_eq!(get_value("ns"), Some(&EncodedValue::Short(-12048)));
        assert_eq!(get_value("i"), Some(&EncodedValue::Int(12540)));
        assert_eq!(get_value("ni"), Some(&EncodedValue::Int(-12540)));
        assert_eq!(get_value("l"), Some(&EncodedValue::Long(43749374797)));
        assert_eq!(get_value("nl"), Some(&EncodedValue::Long(-43749374797)));
        assert_eq!(get_value("f"), Some(&EncodedValue::Float(9.30)));
        assert_eq!(get_value("nf"), Some(&EncodedValue::Float(-9.34)));
        assert_eq!(get_value("d"), Some(&EncodedValue::Double(4374.943749374937)));
        assert_eq!(get_value("nd"), Some(&EncodedValue::Double(-1257.9374937493)));
        assert_eq!(get_value("bo"), Some(&EncodedValue::Boolean(true)));
        assert_eq!(get_value("nbo"), Some(&EncodedValue::Boolean(false)));
        assert_eq!(get_value("c"), Some(&EncodedValue::Char(b'm'.into())));
        assert_eq!(get_value("nullString"), Some(&EncodedValue::Null));
        assert_eq!(get_value("nonNullString"), Some(&EncodedValue::String(DexString::from("fjdljfdlj".to_string()))));
        assert_eq!(get_value("remapper"), Some(&EncodedValue::Null));
        assert_eq!(get_value("r"), Some(&EncodedValue::Null));
        assert_eq!(get_value("array"), Some(&EncodedValue::Null));
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
        use dex::class::Class;
        let validate_interface_methods = |interface: &Class| {
            interface.methods().for_each(|m| {
                assert_has_access_flags!(m, [is_public, is_abstract], format!("interface method: {} doesn't have all attributes", m.name()));
                assert!(m.code().is_none(), "interface method: {} shouldn't have code item", m.name());
            });
        };
        let validate_interface_fields = |interface: &Class| {
            interface.fields().for_each(|f| {
                assert_has_access_flags!(f, [is_public, is_static, is_final], format!("interface field: {} doesn't have all attributes", f.name()));
            });
        };

        let interface = dex.find_class_by_name("LMyInterface;");
        assert!(interface.is_ok());
        let interface = interface.unwrap();
        assert!(interface.is_some());
        let interface = interface.unwrap();
        assert_has_access_flags!(interface, [is_interface]);
        assert_eq!(interface.fields().count(), 2);
        assert_eq!(interface.methods().count(), 2);
        validate_interface_fields(&interface);
        validate_interface_methods(&interface);

        let interface2 = dex.find_class_by_name("LMyInterface2;");
        assert!(interface2.is_ok());
        let interface2 = interface2.unwrap();
        assert!(interface2.is_some());
        let interface2 = interface2.unwrap();
        assert_has_access_flags!(interface2, [is_public, is_interface]);
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
            c.is_synthetic()
        });
        assert!(interface3.is_some());
        let interface3 = interface3.unwrap();
        assert!(interface3.is_ok());
        let interface3 = interface3.unwrap();
        assert_eq!(interface3.methods().count(), 1);
        let method = interface3.methods().take(1).next();
        assert!(method.is_some());
        let method = method.unwrap();
        assert_has_access_flags!(method, [is_static]);
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
        let abstract_class = dex.find_class_by_name("LAbstractClass;");
        assert!(abstract_class.is_ok());
        let abstract_class = abstract_class.unwrap();
        assert!(abstract_class.is_some());
        let abstract_class = abstract_class.unwrap();
        assert_has_access_flags!(abstract_class, [is_abstract]);
        assert_eq!(abstract_class.methods().count(), 4);
        let mut methods = abstract_class.methods().map(|m| m.name()).collect::<Vec<_>>();
        methods.sort();
        let expected = &mut ["<init>", "staticMethod", "abstractMethod", "finalMethod"];
        expected.sort();
        assert_eq!(&methods, expected);
        let abstract_method = abstract_class.methods().find(|m| m.name() == "abstractMethod").unwrap();
        assert!(abstract_method.code().is_none());
        assert_has_access_flags!(abstract_method, [is_public, is_abstract]);
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
        let enum_class = dex.find_class_by_name("LEnumClass;");
        assert!(enum_class.is_ok());
        let enum_class = enum_class.unwrap();
        assert!(enum_class.is_some());
        let enum_class = enum_class.unwrap();
        assert_has_access_flags!(enum_class, [is_enum]);

        let sunday = enum_class.fields().find(|f| f.name() == "SUNDAY");
        assert!(sunday.is_some());
        let sunday = sunday.unwrap();
        assert_has_access_flags!(sunday, [is_enum]);
    }
);

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
        let class = dex.find_class_by_name("LMain;").unwrap().unwrap();
        assert_eq!(class.direct_methods().len(), 9);
        assert_eq!(class.virtual_methods().len(), 21);

        let find = |name, params: &[&str], return_type: &str| {
            let method = class.methods().find(|m| {
                m.name() == name && 
                    m.params().iter().map(|s| s.type_descriptor()).eq(params.iter()) &&
                    m.return_type() == return_type
            });
            assert!(method.is_some(), "method: {}, params: {:?}, return_type: {}", name, params, return_type);
            let method = method.unwrap();
            method
        };

        let find_id_item = |id: MethodId| {
            dex.method_ids().find(|m| if let Ok(m) = m {
                m.id() == id
            } else {
                false
            })
        };

        let default_method = find("defaultMethod", &[], "V");
        assert!(default_method.code().is_some());
        assert!(default_method.access_flags().is_empty());
        assert_eq!(default_method.shorty(), "V");
        assert!(find_id_item(default_method.id()).is_some());

        let final_method = find("finalMethod", &[], "V");
        assert!(final_method.code().is_some());
        assert_has_access_flags!(final_method, [is_final]);
        assert_eq!(final_method.shorty(), "V");
        assert!(find_id_item(final_method.id()).is_some());

        let static_method = find("staticMethod", &[], "V");
        assert!(static_method.code().is_some());
        assert_has_access_flags!(static_method, [is_static]);
        assert_eq!(static_method.shorty(), "V");
        assert!(find_id_item(static_method.id()).is_some());

        let public_method = find("publicMethod", &[], "V");
        assert!(public_method.code().is_some());
        assert_has_access_flags!(public_method, [is_public]);
        assert_eq!(public_method.shorty(), "V");
        assert!(find_id_item(public_method.id()).is_some());

        let private_method = find("privateMethod", &[], "V");
        assert!(private_method.code().is_some());
        assert_has_access_flags!(private_method, [is_private]);
        assert_eq!(private_method.shorty(), "V");
        assert!(find_id_item(private_method.id()).is_some());

        let protected_method = find("protectedMethod", &[], "V");
        assert!(protected_method.code().is_some());
        assert_has_access_flags!(protected_method, [is_protected]);
        assert_eq!(protected_method.shorty(), "V");
        assert!(find_id_item(protected_method.id()).is_some());


        let primitive_return_method = find("primitiveReturnMethod", &[], "I");
        assert!(primitive_return_method.code().is_some());
        assert!(primitive_return_method.access_flags().is_empty());
        assert_eq!(primitive_return_method.shorty(), "I");
        assert!(find_id_item(primitive_return_method.id()).is_some());

        let class_return_method = find("classReturnMethod", &[], "Ljava/lang/String;");
        assert!(primitive_return_method.code().is_some());
        assert!(class_return_method.access_flags().is_empty());
        assert_eq!(class_return_method.shorty(), "L");
        assert!(find_id_item(class_return_method.id()).is_some());

        let array_return_method = find("arrayReturnMethod", &[], "[J");
        assert!(array_return_method.code().is_some());
        assert!(array_return_method.access_flags().is_empty());
        assert_eq!(array_return_method.shorty(), "L");
        assert!(find_id_item(array_return_method.id()).is_some());

        let object_array_return_method = find("objectArrayReturnMethod", &[], "[Ljava/lang/String;");
        assert!(object_array_return_method.code().is_some());
        assert!(array_return_method.access_flags().is_empty());
        assert!(object_array_return_method.access_flags().is_empty());
        assert_eq!(object_array_return_method.shorty(), "L");
        assert!(find_id_item(object_array_return_method.id()).is_some());

        let enum_return_method = find("enumReturnMethod", &[], "LDay;");
        assert!(enum_return_method.code().is_some());
        assert!(enum_return_method.access_flags().is_empty());
        assert_eq!(enum_return_method.shorty(), "L");
        assert!(find_id_item(enum_return_method.id()).is_some());


        let primitive_params_method = find("primitiveParams", &["C", "S", "B", "I", "J", "Z", "D", "F"], "I");
        assert!(primitive_params_method.code().is_some());
        assert!(primitive_params_method.access_flags().is_empty());
        assert_eq!(primitive_params_method.shorty(), "ICSBIJZDF");
        assert!(find_id_item(primitive_params_method.id()).is_some());
        
        let class_params_method = find("classParams", &["Ljava/lang/String;", "Ljava/lang/String;"], "Ljava/lang/String;");
        assert!(class_params_method.code().is_some());
        assert!(class_params_method.access_flags().is_empty());
        assert_eq!(class_params_method.shorty(), "LLL");
        assert!(find_id_item(class_params_method.id()).is_some());

        let enum_params_method = find("enumParam", &["LDay;"], "V");
        assert!(enum_params_method.code().is_some());
        assert!(enum_params_method.access_flags().is_empty());
        assert_eq!(enum_params_method.shorty(), "VL");
        assert!(find_id_item(enum_params_method.id()).is_some());
        
        let primitive_array_params_method = find("primitiveArrayParam", &["[J"], "V");
        assert!(primitive_array_params_method.code().is_some());
        assert!(primitive_array_params_method.access_flags().is_empty());
        assert_eq!(primitive_array_params_method.shorty(), "VL");
        assert!(find_id_item(primitive_array_params_method.id()).is_some());

        let object_array_params_method = find("objectArrayParam", &["[Ljava/lang/String;"], "V");
        assert!(object_array_params_method.code().is_some());
        assert!(object_array_params_method.access_flags().is_empty());
        assert_eq!(object_array_params_method.shorty(), "VL");
        assert!(find_id_item(object_array_params_method.id()).is_some());

        let interface_params_method = find("interfaceParam", &["LMyInterface;"], "V");
        assert!(interface_params_method.code().is_some());
        assert!(interface_params_method.access_flags().is_empty());
        assert_eq!(interface_params_method.shorty(), "VL");
        assert!(find_id_item(interface_params_method.id()).is_some());

        let generic_params_method  = find("genericParamsMethod1", &["Ljava/util/List;", "I"], "V");
        assert!(generic_params_method.code().is_some());
        assert_has_access_flags!(generic_params_method, [is_private]);
        assert_eq!(generic_params_method.shorty(), "VLI");
        assert!(find_id_item(generic_params_method.id()).is_some());

        let generic_params_method  = find("genericParamsMethod2", &["Ljava/lang/Object;", "I"], "V");
        assert!(generic_params_method.code().is_some());
        assert_has_access_flags!(generic_params_method, [is_private]);
        assert_eq!(generic_params_method.shorty(), "VLI");
        assert!(find_id_item(generic_params_method.id()).is_some());

        let generic_params_method  = find("genericParamsMethod3", &["Ljava/util/List;", "I"], "V");
        assert!(generic_params_method.code().is_some());
        assert_has_access_flags!(generic_params_method, [is_private]);
        assert_eq!(generic_params_method.shorty(), "VLI");
        assert!(find_id_item(generic_params_method.id()).is_some());

        let generic_params_method  = find("genericParamsMethod4", &["Ljava/util/List;", "I"], "V");
        assert!(generic_params_method.code().is_some());
        assert_has_access_flags!(generic_params_method, [is_private]);
        assert_eq!(generic_params_method.shorty(), "VLI");
        assert!(find_id_item(generic_params_method.id()).is_some());


        let generic_params_method  = find("genericParamWithExtendsClauseMethod", &["LSuperClass;"], "V");
        assert!(generic_params_method.code().is_some());
        assert_has_access_flags!(generic_params_method, [is_private]);
        assert_eq!(generic_params_method.shorty(), "VL");
        assert!(find_id_item(generic_params_method.id()).is_some());

        let generic_params_method  = find("genericParamWithMultipleExtendsClauseMethod", &["LSuperClass;"], "V");
        assert!(generic_params_method.code().is_some());
        assert_has_access_flags!(generic_params_method, [is_private]);
        assert_eq!(generic_params_method.shorty(), "VL");
        assert!(find_id_item(generic_params_method.id()).is_some());


        let varargs_method = find("varargsMethod", &["[Ljava/lang/String;"], "I");
        assert!(varargs_method.code().is_some());
        assert_has_access_flags!(varargs_method, [is_public, is_varargs]);
        assert_eq!(varargs_method.shorty(), "IL");
        assert!(find_id_item(varargs_method.id()).is_some());


        let super_method = find("superMethod", &["Ljava/lang/String;"], "I");
        assert!(super_method.code().is_some());
        assert!(super_method.access_flags().is_empty());
        assert_eq!(super_method.shorty(), "IL");
        assert!(find_id_item(super_method.id()).is_some());

        let super_method2 = class.fields().find(|m| m.name() == "superMethod2");
        assert!(super_method2.is_none(), "super method 2 is not overriden, so it shouldn't be there");


        let interface_method = find("interfaceMethod", &["I", "Ljava/lang/String;"], "Ljava/lang/String;");
        assert!(interface_method.code().is_some());
        assert_has_access_flags!(interface_method, [is_public]);
        assert_eq!(interface_method.shorty(), "LIL");
        assert!(find_id_item(interface_method.id()).is_some());


        let native_method = find("nativeMethod", &["I", "Ljava/lang/String;"], "Ljava/lang/String;");
        assert!(native_method.code().is_none());
        assert_has_access_flags!(native_method, [is_public, is_native]);
        assert_eq!(native_method.shorty(), "LIL");
        assert!(find_id_item(native_method.id()).is_some());

        let abstract_method = find("abstractMethod", &["I"], "I");
        assert!(abstract_method.code().is_none());
        assert_has_access_flags!(abstract_method, [is_abstract]);
        assert_eq!(abstract_method.shorty(), "II");
        assert!(find_id_item(abstract_method.id()).is_some());

        let synchronized_method = find("synchronizedMethod", &["I"], "I");
        assert!(synchronized_method.code().is_some());
        assert_has_access_flags!(synchronized_method, [is_declared_synchronized]);
        assert_eq!(synchronized_method.shorty(), "II");
        assert!(find_id_item(synchronized_method.id()).is_some());
    }
);

test!(
    test_builtin_types,
    {
        "BuiltInTypes.java" => r#"
            public class BuiltInTypes {
               boolean returnsBoolean() { return false; }
               byte returnsByte() { return 0; }
               short returnsShort() { return 0; }
               char returnsChar() { return 0; }
               int returnsInt() { return 0; }
               long returnsLong() { return 0; }
               float returnsFloat() { return 0; }
               double returnsDouble() { return 0; }
               // This is technically not needed, as the void type is always present
               void returnsVoid() {}
            }
        "#
    },
    |dex: dex::Dex<_>| {
        let builtin_class = dex.find_class_by_name("LBuiltInTypes;").unwrap().unwrap();

        let find_type = |name: &str| {
            dex.types().find(|t| {
                if let Ok(t) = t {
                    t.to_string() == name
                } else {
                    false
                }
            })
        };

        let boolean_type = find_type("Z");
        assert!(boolean_type.is_some());

        let byte_type = find_type("B");
        assert!(byte_type.is_some());

        let short_type = find_type("S");
        assert!(short_type.is_some());

        let char_type = find_type("C");
        assert!(char_type.is_some());

        let int_type = find_type("I");
        assert!(int_type.is_some());

        let long_type = find_type("J");
        assert!(long_type.is_some());

        let float_type = find_type("F");
        assert!(float_type.is_some());

        let double_type = find_type("D");
        assert!(double_type.is_some());

        let void_type = find_type("V");
        assert!(void_type.is_some());

    }
);

#[test]
fn test_iterators() {
    use dex::DexReader;
    let dex = DexReader::from_file("resources/classes.dex").expect("can't open dex");
    for jtype in dex.types() {
        assert!(jtype.is_ok());
    }

    for proto_id_item in dex.proto_ids() {
        assert!(proto_id_item.is_ok());
    }

    for field_id_item in dex.field_ids() {
        assert!(field_id_item.is_ok());
    }

    for method_id_item in dex.method_ids() {
        assert!(method_id_item.is_ok());
    }

    for method_handle_item in dex.method_handles() {
        assert!(method_handle_item.is_ok());
    }
}
