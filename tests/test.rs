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
    ($test_name: ident, $({ $fname:expr => $code:expr }),+,$test_func:expr) => {
        #[test]
        fn $test_name() {
            use dex_parser::DexBuilder;
            let mut builder = TestBuilder::new();
            $(
               builder.add_src($fname, $code);
            )*
            let dex_path = builder.compile();
            let dex = DexBuilder::from_file(dex_path.as_path());
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
        "Test.java" =>
        r#"
            class Test {
             public static void main(String[] args) {
                System.out.println("1 + 1 = " + 1 + 1);
             }
            }
       "#
    }
);

test!(
    test_strings_len_match,
    {
        "Test.java" =>
        r#"
            class Test {}
        "#
    },
    |dex: dex_parser::Dex<_>| {
        let len = dex.strings().count();
        assert_eq!(len, 6);
    }
);
