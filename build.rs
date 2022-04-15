use clap::CommandFactory;

include!("src/cli.rs");

fn check_output(o: std::process::Output) {
    if !o.status.success() {
        if let Ok(s) = String::from_utf8(o.stdout) {
            eprintln!("{}", s);
        }
        panic!("Command failed");
    }
}

fn generate_shell_completion() {
    let mut cmd = Args::command();
    cmd.set_bin_name("zscdoc");

    let out = std::env::var_os("OUT_DIR").unwrap();
    let out = std::path::Path::new(&out).join("completions");
    fs_extra::dir::create_all(&out, true).expect("expected to create dir");
    for s in clap_complete::shells::Shell::value_variants() {
        clap_complete::generate_to(*s, &mut cmd, "zscdoc", &out).unwrap();
    }
}

pub fn main() {
    let profile = std::env::var("PROFILE").unwrap();
    if profile.as_str() == "debug" {
        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:rerun-if-changed=src/cli.rs");

        generate_shell_completion();
        return;
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/cli.rs");
    println!("cargo:rerun-if-changed=web_stuff/src");
    println!("cargo:rerun-if-changed=web_stuff/fonts");
    println!("cargo:rerun-if-changed=web_stuff/package-lock.json");
    println!("cargo:rerun-if-changed=web_stuff/package.json");
    println!("cargo:rerun-if-changed=web_stuff/tsconfig.json");
    println!("cargo:rerun-if-changed=web_stuff/webpack.config.js");

    generate_shell_completion();

    let dir_options = fs_extra::dir::CopyOptions::new();

    let out = std::env::var_os("OUT_DIR").unwrap();
    let out = std::path::Path::new(&out);
    let dest = out.join("web_stuff");

    fs_extra::dir::create_all(&dest, true).expect("expected to create dir");
    fs_extra::copy_items(
        &[
            "web_stuff/src",
            "web_stuff/fonts",
            "web_stuff/package-lock.json",
            "web_stuff/package.json",
            "web_stuff/tsconfig.json",
            "web_stuff/webpack.config.js",
        ],
        &dest,
        &dir_options,
    )
    .expect("expected to copy dir");

    std::env::set_current_dir(&dest).expect("expected to cd to dest dir");

    eprintln!("dest: {:?}", dest);

    check_output(
        std::process::Command::new("npm")
            .args(["ci"])
            .output()
            .expect("expected to install packages"),
    );

    check_output(
        std::process::Command::new("npm")
            .args(["--production", "run", "build"])
            .output()
            .expect("expected to build web_stuff"),
    );
}
