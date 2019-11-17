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

// TODO: add tests for interface fields, generic fields, initial values, annotations on fields
test!(
    test_fields,
    {
        "Main.java" => r#"
          class Main {
              public static int staticVar = 42;
              final double finalVar = 32.0d;
              private String privateField;
              public String publicField;
              protected String protectedField;
              int[] arrayField;
              Day enumField;
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
        assert_eq!(class.instance_fields().count(), 6);
        assert_eq!(class.fields().count(), 7);
        let find = |name| { 
            let field = class.fields().find(|f| f.name() == name);
            assert!(field.is_some());
            field.unwrap()
        };
        let static_field = find(&"staticVar");
        assert!(static_field.access_flags().contains(AccessFlags::STATIC));
        assert!(static_field.access_flags().contains(AccessFlags::PUBLIC));
        assert_eq!(static_field.jtype(), &"I");

        let final_field = find(&"finalVar");
        assert!(final_field.access_flags().contains(AccessFlags::FINAL));
        assert_eq!(final_field.jtype(), &"D");

        let protected_field = find(&"protectedField");
        assert!(protected_field.access_flags().contains(AccessFlags::PROTECTED));
        assert_eq!(protected_field.jtype(), &"Ljava/lang/String;");

        let private_field = find(&"privateField");
        assert!(private_field.access_flags().contains(AccessFlags::PRIVATE));

        let public_field = find(&"publicField");
        assert!(public_field.access_flags().contains(AccessFlags::PUBLIC));

        let array_field = find(&"arrayField");
        assert_eq!(array_field.jtype(), &"[I");

        // TODO: find out why d8 fails with warning:
        // d8 is from build-tools:29.0.2
        // Type `java.lang.Enum` was not found, it is required for default or static interface
        // methods desugaring of `Day Day.valueOf(java.lang.String)`
        // let enum_field = find(&"enumField");
        // assert!(enum_field.access_flags().contains(AccessFlags::ENUM));
        
    }
);

// TODO:  test interfaces, enums, abstract classes

test!(
    test_class_methods,
    {
        "Main.java" => r#"
            import java.util.List;
            class Main extends SuperClass implements MyInterface {
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
              Day enumReturnMethod() { return Day.SUNDAY; }

              // params
              int primitiveParams(char u, short v, byte w, int x, long y, boolean z, double a, float b) { return 0; }
              String classParams(String x, String y) { return "22"; }
              void enumParam(Day day) {}
              void interfaceParam(MyInterface instance) {}

              // overriden method
              @Override int superMethod(String y) { return 2; }
              
              // interface method
              public String interfaceMethod(int x, String y) { return y + x; }

              private <T> void processChanges(List<T> myList, int k) {}

              private <T> void processChanges(T typeParam, int k) {}
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
        assert_eq!(class.methods().count(), 19);
        assert_eq!(class.direct_methods().count(), 5);
        assert_eq!(class.virtual_methods().count(), 14);
        // TODO: test method access flags, params, annotations etc.
    }
);
