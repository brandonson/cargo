use std::fs::{self, File};
use std::io::prelude::*;
use std::thread;

use support::{project, execs, main_file};
use support::{COMPILING, RUNNING};
use support::paths::{self, CargoPathExt};
use hamcrest::{assert_that, existing_file};
use cargo::util::process;

fn setup() {
}

test!(cargo_compile_with_nested_deps_shorthand {
    let p = project("foo")
        .file("Cargo.toml", r#"
            [project]

            name = "foo"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [dependencies.bar]

            version = "0.5.0"
            path = "bar"

            [[bin]]

            name = "foo"
        "#)
        .file("src/foo.rs",
              &main_file(r#""{}", bar::gimme()"#, &["bar"]))
        .file("bar/Cargo.toml", r#"
            [project]

            name = "bar"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [dependencies.baz]

            version = "0.5.0"
            path = "baz"

            [lib]

            name = "bar"
        "#)
        .file("bar/src/bar.rs", r#"
            extern crate baz;

            pub fn gimme() -> String {
                baz::gimme()
            }
        "#)
        .file("bar/baz/Cargo.toml", r#"
            [project]

            name = "baz"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [lib]

            name = "baz"
        "#)
        .file("bar/baz/src/baz.rs", r#"
            pub fn gimme() -> String {
                "test passed".to_string()
            }
        "#);

    assert_that(p.cargo_process("build"),
        execs().with_status(0)
               .with_stdout(&format!("{} baz v0.5.0 ({})\n\
                                     {} bar v0.5.0 ({})\n\
                                     {} foo v0.5.0 ({})\n",
                                    COMPILING, p.url(),
                                    COMPILING, p.url(),
                                    COMPILING, p.url())));

    assert_that(&p.bin("foo"), existing_file());

    assert_that(process(&p.bin("foo")),
                execs().with_stdout("test passed\n").with_status(0));

    println!("cleaning");
    assert_that(p.cargo("clean"),
                execs().with_stdout("").with_status(0));
    println!("building baz");
    assert_that(p.cargo("build").arg("-p").arg("baz"),
                execs().with_status(0)
                       .with_stdout(&format!("{} baz v0.5.0 ({})\n",
                                            COMPILING, p.url())));
    println!("building foo");
    assert_that(p.cargo("build")
                 .arg("-p").arg("foo"),
                execs().with_status(0)
                       .with_stdout(&format!("{} bar v0.5.0 ({})\n\
                                             {} foo v0.5.0 ({})\n",
                                            COMPILING, p.url(),
                                            COMPILING, p.url())));
});

test!(cargo_compile_with_root_dev_deps {
    let p = project("foo")
        .file("Cargo.toml", r#"
            [project]

            name = "foo"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [dev-dependencies.bar]

            version = "0.5.0"
            path = "../bar"

            [[bin]]
            name = "foo"
        "#)
        .file("src/main.rs",
              &main_file(r#""{}", bar::gimme()"#, &["bar"]));
    let p2 = project("bar")
        .file("Cargo.toml", r#"
            [package]

            name = "bar"
            version = "0.5.0"
            authors = ["wycats@example.com"]
        "#)
        .file("src/lib.rs", r#"
            pub fn gimme() -> &'static str {
                "zoidberg"
            }
        "#);

    p2.build();
    assert_that(p.cargo_process("build"),
                execs().with_status(101))
});

test!(cargo_compile_with_root_dev_deps_with_testing {
    let p = project("foo")
        .file("Cargo.toml", r#"
            [project]

            name = "foo"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [dev-dependencies.bar]

            version = "0.5.0"
            path = "../bar"

            [[bin]]
            name = "foo"
        "#)
        .file("src/main.rs",
              &main_file(r#""{}", bar::gimme()"#, &["bar"]));
    let p2 = project("bar")
        .file("Cargo.toml", r#"
            [package]

            name = "bar"
            version = "0.5.0"
            authors = ["wycats@example.com"]
        "#)
        .file("src/lib.rs", r#"
            pub fn gimme() -> &'static str {
                "zoidberg"
            }
        "#);

    p2.build();
    assert_that(p.cargo_process("test"),
        execs().with_stdout(&format!("\
{compiling} [..] v0.5.0 ({url})
{compiling} [..] v0.5.0 ({url})
{running} target[..]foo-[..]

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured

", compiling = COMPILING, url = p.url(), running = RUNNING)));
});

test!(cargo_compile_with_transitive_dev_deps {
    let p = project("foo")
        .file("Cargo.toml", r#"
            [project]

            name = "foo"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [dependencies.bar]

            version = "0.5.0"
            path = "bar"

            [[bin]]

            name = "foo"
        "#)
        .file("src/foo.rs",
              &main_file(r#""{}", bar::gimme()"#, &["bar"]))
        .file("bar/Cargo.toml", r#"
            [project]

            name = "bar"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [dev-dependencies.baz]

            git = "git://example.com/path/to/nowhere"

            [lib]

            name = "bar"
        "#)
        .file("bar/src/bar.rs", r#"
            pub fn gimme() -> &'static str {
                "zoidberg"
            }
        "#);

    assert_that(p.cargo_process("build"),
        execs().with_stdout(&format!("{} bar v0.5.0 ({})\n\
                                     {} foo v0.5.0 ({})\n",
                                    COMPILING, p.url(),
                                    COMPILING, p.url())));

    assert_that(&p.bin("foo"), existing_file());

    assert_that(process(&p.bin("foo")),
                execs().with_stdout("zoidberg\n"));
});

test!(no_rebuild_dependency {
    let mut p = project("foo");
    p = p
        .file("Cargo.toml", r#"
            [project]

            name = "foo"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [[bin]] name = "foo"
            [dependencies.bar] path = "bar"
        "#)
        .file("src/foo.rs", r#"
            extern crate bar;
            fn main() { bar::bar() }
        "#)
        .file("bar/Cargo.toml", r#"
            [project]

            name = "bar"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [lib] name = "bar"
        "#)
        .file("bar/src/bar.rs", r#"
            pub fn bar() {}
        "#);
    // First time around we should compile both foo and bar
    assert_that(p.cargo_process("build"),
                execs().with_stdout(&format!("{} bar v0.5.0 ({})\n\
                                             {} foo v0.5.0 ({})\n",
                                            COMPILING, p.url(),
                                            COMPILING, p.url())));
    // This time we shouldn't compile bar
    assert_that(p.cargo("build"),
                execs().with_stdout(""));
    p.root().move_into_the_past().unwrap();

    p.build(); // rebuild the files (rewriting them in the process)
    assert_that(p.cargo("build"),
                execs().with_stdout(&format!("{} bar v0.5.0 ({})\n\
                                             {} foo v0.5.0 ({})\n",
                                            COMPILING, p.url(),
                                            COMPILING, p.url())));
});

test!(deep_dependencies_trigger_rebuild {
    let mut p = project("foo");
    p = p
        .file("Cargo.toml", r#"
            [project]

            name = "foo"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [[bin]]
            name = "foo"
            [dependencies.bar]
            path = "bar"
        "#)
        .file("src/foo.rs", r#"
            extern crate bar;
            fn main() { bar::bar() }
        "#)
        .file("bar/Cargo.toml", r#"
            [project]

            name = "bar"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [lib]
            name = "bar"
            [dependencies.baz]
            path = "../baz"
        "#)
        .file("bar/src/bar.rs", r#"
            extern crate baz;
            pub fn bar() { baz::baz() }
        "#)
        .file("baz/Cargo.toml", r#"
            [project]

            name = "baz"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [lib]
            name = "baz"
        "#)
        .file("baz/src/baz.rs", r#"
            pub fn baz() {}
        "#);
    assert_that(p.cargo_process("build"),
                execs().with_stdout(&format!("{} baz v0.5.0 ({})\n\
                                             {} bar v0.5.0 ({})\n\
                                             {} foo v0.5.0 ({})\n",
                                            COMPILING, p.url(),
                                            COMPILING, p.url(),
                                            COMPILING, p.url())));
    assert_that(p.cargo("build"),
                execs().with_stdout(""));

    // Make sure an update to baz triggers a rebuild of bar
    //
    // We base recompilation off mtime, so sleep for at least a second to ensure
    // that this write will change the mtime.
    thread::sleep_ms(1000);
    File::create(&p.root().join("baz/src/baz.rs")).unwrap().write_all(br#"
        pub fn baz() { println!("hello!"); }
    "#).unwrap();
    assert_that(p.cargo("build"),
                execs().with_stdout(&format!("{} baz v0.5.0 ({})\n\
                                             {} bar v0.5.0 ({})\n\
                                             {} foo v0.5.0 ({})\n",
                                            COMPILING, p.url(),
                                            COMPILING, p.url(),
                                            COMPILING, p.url())));

    // Make sure an update to bar doesn't trigger baz
    thread::sleep_ms(1000);
    File::create(&p.root().join("bar/src/bar.rs")).unwrap().write_all(br#"
        extern crate baz;
        pub fn bar() { println!("hello!"); baz::baz(); }
    "#).unwrap();
    assert_that(p.cargo("build"),
                execs().with_stdout(&format!("{} bar v0.5.0 ({})\n\
                                             {} foo v0.5.0 ({})\n",
                                            COMPILING, p.url(),
                                            COMPILING, p.url())));

});

test!(no_rebuild_two_deps {
    let mut p = project("foo");
    p = p
        .file("Cargo.toml", r#"
            [project]

            name = "foo"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [[bin]]
            name = "foo"
            [dependencies.bar]
            path = "bar"
            [dependencies.baz]
            path = "baz"
        "#)
        .file("src/foo.rs", r#"
            extern crate bar;
            fn main() { bar::bar() }
        "#)
        .file("bar/Cargo.toml", r#"
            [project]

            name = "bar"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [lib]
            name = "bar"
            [dependencies.baz]
            path = "../baz"
        "#)
        .file("bar/src/bar.rs", r#"
            pub fn bar() {}
        "#)
        .file("baz/Cargo.toml", r#"
            [project]

            name = "baz"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [lib]
            name = "baz"
        "#)
        .file("baz/src/baz.rs", r#"
            pub fn baz() {}
        "#);
    assert_that(p.cargo_process("build"),
                execs().with_stdout(&format!("{} baz v0.5.0 ({})\n\
                                             {} bar v0.5.0 ({})\n\
                                             {} foo v0.5.0 ({})\n",
                                            COMPILING, p.url(),
                                            COMPILING, p.url(),
                                            COMPILING, p.url())));
    assert_that(&p.bin("foo"), existing_file());
    assert_that(p.cargo("build"),
                execs().with_stdout(""));
    assert_that(&p.bin("foo"), existing_file());
});

test!(nested_deps_recompile {
    let p = project("foo")
        .file("Cargo.toml", r#"
            [project]

            name = "foo"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [dependencies.bar]

            version = "0.5.0"
            path = "src/bar"

            [[bin]]

            name = "foo"
        "#)
        .file("src/foo.rs",
              &main_file(r#""{}", bar::gimme()"#, &["bar"]))
        .file("src/bar/Cargo.toml", r#"
            [project]

            name = "bar"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [lib]

            name = "bar"
        "#)
        .file("src/bar/src/bar.rs", "pub fn gimme() {}");
    let bar = p.url();

    assert_that(p.cargo_process("build"),
                execs().with_stdout(&format!("{} bar v0.5.0 ({})\n\
                                             {} foo v0.5.0 ({})\n",
                                            COMPILING, bar,
                                            COMPILING, p.url())));
    p.root().move_into_the_past().unwrap();

    File::create(&p.root().join("src/foo.rs")).unwrap().write_all(br#"
        fn main() {}
    "#).unwrap();

    // This shouldn't recompile `bar`
    assert_that(p.cargo("build"),
                execs().with_stdout(&format!("{} foo v0.5.0 ({})\n",
                                            COMPILING, p.url())));
});

test!(error_message_for_missing_manifest {
    let p = project("foo")
        .file("Cargo.toml", r#"
            [project]

            name = "foo"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [dependencies.bar]

            path = "src/bar"

            [lib]

            name = "foo"
        "#)
       .file("src/bar/not-a-manifest", "");

    assert_that(p.cargo_process("build"),
                execs()
                .with_status(101)
                .with_stderr(&format!("\
Unable to update file://[..]

Caused by:
  Could not find `Cargo.toml` in `{}`
", p.root().join("src").join("bar").display())));

});

test!(override_relative {
    let bar = project("bar")
        .file("Cargo.toml", r#"
            [package]

            name = "bar"
            version = "0.5.0"
            authors = ["wycats@example.com"]
        "#)
       .file("src/lib.rs", "");

    fs::create_dir(&paths::root().join(".cargo")).unwrap();
    File::create(&paths::root().join(".cargo/config")).unwrap()
         .write_all(br#"paths = ["bar"]"#).unwrap();

    let p = project("foo")
        .file("Cargo.toml", &format!(r#"
            [package]

            name = "foo"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [dependencies.bar]
            path = '{}'
        "#, bar.root().display()))
       .file("src/lib.rs", "");
    bar.build();
    assert_that(p.cargo_process("build").arg("-v"), execs().with_status(0));

});

test!(override_self {
    let bar = project("bar")
        .file("Cargo.toml", r#"
            [package]

            name = "bar"
            version = "0.5.0"
            authors = ["wycats@example.com"]
        "#)
       .file("src/lib.rs", "");

    let p = project("foo");
    let root = p.root().clone();
    let p = p
        .file(".cargo/config", &format!(r#"
            paths = ['{}']
        "#, root.display()))
        .file("Cargo.toml", &format!(r#"
            [package]

            name = "foo"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [dependencies.bar]
            path = '{}'

        "#, bar.root().display()))
       .file("src/lib.rs", "")
       .file("src/main.rs", "fn main() {}");

    bar.build();
    assert_that(p.cargo_process("build"), execs().with_status(0));

});

test!(override_path_dep {
    let bar = project("bar")
       .file("p1/Cargo.toml", r#"
            [package]
            name = "p1"
            version = "0.5.0"
            authors = []

            [dependencies.p2]
            path = "../p2"
       "#)
       .file("p1/src/lib.rs", "")
       .file("p2/Cargo.toml", r#"
            [package]
            name = "p2"
            version = "0.5.0"
            authors = []
       "#)
       .file("p2/src/lib.rs", "");

    let p = project("foo")
        .file(".cargo/config", &format!(r#"
            paths = ['{}', '{}']
        "#, bar.root().join("p1").display(),
            bar.root().join("p2").display()))
        .file("Cargo.toml", &format!(r#"
            [package]

            name = "foo"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [dependencies.p2]
            path = '{}'

        "#, bar.root().join("p2").display()))
       .file("src/lib.rs", "");

    bar.build();
    assert_that(p.cargo_process("build").arg("-v"),
                execs().with_status(0));

});

test!(path_dep_build_cmd {
    let p = project("foo")
        .file("Cargo.toml", r#"
            [project]

            name = "foo"
            version = "0.5.0"
            authors = ["wycats@example.com"]

            [dependencies.bar]

            version = "0.5.0"
            path = "bar"

            [[bin]]

            name = "foo"
        "#)
        .file("src/foo.rs",
              &main_file(r#""{}", bar::gimme()"#, &["bar"]))
        .file("bar/Cargo.toml", r#"
            [project]

            name = "bar"
            version = "0.5.0"
            authors = ["wycats@example.com"]
            build = "build.rs"

            [lib]

            name = "bar"
        "#)
        .file("bar/build.rs", r#"
            use std::fs;
            fn main() {
                fs::copy("src/bar.rs.in", "src/bar.rs").unwrap();
            }
        "#)
        .file("bar/src/bar.rs.in", r#"
            pub fn gimme() -> i32 { 0 }
        "#);

    p.build();
    p.root().join("bar").move_into_the_past().unwrap();

    assert_that(p.cargo("build"),
        execs().with_stdout(&format!("{} bar v0.5.0 ({})\n\
                                     {} foo v0.5.0 ({})\n",
                                    COMPILING, p.url(),
                                    COMPILING, p.url())));

    assert_that(&p.bin("foo"), existing_file());

    assert_that(process(&p.bin("foo")),
                execs().with_stdout("0\n"));

    // Touching bar.rs.in should cause the `build` command to run again.
    {
        let file = fs::File::create(&p.root().join("bar/src/bar.rs.in"));
        file.unwrap().write_all(br#"pub fn gimme() -> i32 { 1 }"#).unwrap();
    }

    assert_that(p.cargo("build"),
        execs().with_stdout(&format!("{} bar v0.5.0 ({})\n\
                                     {} foo v0.5.0 ({})\n",
                                    COMPILING, p.url(),
                                    COMPILING, p.url())));

    assert_that(process(&p.bin("foo")),
                execs().with_stdout("1\n"));
});

test!(dev_deps_no_rebuild_lib {
    let p = project("foo")
        .file("Cargo.toml", r#"
            [project]
                name = "foo"
                version = "0.5.0"
                authors = []

            [dev-dependencies.bar]
                path = "bar"

            [lib]
                name = "foo"
                doctest = false
        "#)
        .file("src/lib.rs", r#"
            #[cfg(test)] extern crate bar;
            #[cfg(not(test))] fn foo() { env!("FOO"); }
        "#)
        .file("bar/Cargo.toml", r#"
            [package]

            name = "bar"
            version = "0.5.0"
            authors = ["wycats@example.com"]
        "#)
        .file("bar/src/lib.rs", "pub fn bar() {}");
    p.build();
    assert_that(p.cargo("build")
                 .env("FOO", "bar"),
                execs().with_status(0)
                       .with_stdout(&format!("{} foo v0.5.0 ({})\n",
                                            COMPILING, p.url())));

    assert_that(p.cargo("test"),
                execs().with_status(0)
                       .with_stdout(&format!("\
{} [..] v0.5.0 ({})
{} [..] v0.5.0 ({})
Running target[..]foo-[..]

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured

", COMPILING, p.url(), COMPILING, p.url())));
});

test!(custom_target_no_rebuild {
    let p = project("foo")
        .file("Cargo.toml", r#"
            [project]
            name = "foo"
            version = "0.5.0"
            authors = []
            [dependencies]
            a = { path = "a" }
        "#)
        .file("src/lib.rs", "")
        .file("a/Cargo.toml", r#"
            [project]
            name = "a"
            version = "0.5.0"
            authors = []
        "#)
        .file("a/src/lib.rs", "")
        .file("b/Cargo.toml", r#"
            [project]
            name = "b"
            version = "0.5.0"
            authors = []
            [dependencies]
            a = { path = "../a" }
        "#)
        .file("b/src/lib.rs", "");
    p.build();
    assert_that(p.cargo("build"),
                execs().with_status(0)
                       .with_stdout(&format!("\
{compiling} a v0.5.0 ([..])
{compiling} foo v0.5.0 ([..])
", compiling = COMPILING)));

    assert_that(p.cargo("build")
                 .arg("--manifest-path=b/Cargo.toml")
                 .env("CARGO_TARGET_DIR", "target"),
                execs().with_status(0)
                       .with_stdout(&format!("\
{compiling} b v0.5.0 ([..])
", compiling = COMPILING)));
});

test!(override_and_depend {
    let p = project("foo")
        .file("a/a1/Cargo.toml", r#"
            [project]
            name = "a1"
            version = "0.5.0"
            authors = []
            [dependencies]
            a2 = { path = "../a2" }
        "#)
        .file("a/a1/src/lib.rs", "")
        .file("a/a2/Cargo.toml", r#"
            [project]
            name = "a2"
            version = "0.5.0"
            authors = []
        "#)
        .file("a/a2/src/lib.rs", "")
        .file("b/Cargo.toml", r#"
            [project]
            name = "b"
            version = "0.5.0"
            authors = []
            [dependencies]
            a1 = { path = "../a/a1" }
            a2 = { path = "../a/a2" }
        "#)
        .file("b/src/lib.rs", "")
        .file("b/.cargo/config", r#"
            paths = ["../a"]
        "#);
    p.build();
    assert_that(p.cargo("build").cwd(p.root().join("b")),
                execs().with_status(0)
                       .with_stdout(&format!("\
{compiling} a2 v0.5.0 ([..])
{compiling} a1 v0.5.0 ([..])
{compiling} b v0.5.0 ([..])
", compiling = COMPILING)));
});
