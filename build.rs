fn main() {
    let code = std::process::Command::new("glib-compile-schemas")
        .arg("data/schemas")
        .status()
        .expect("Could not find glib-compile-schemas in PATH");

    for entry in std::path::Path::new("data/schemas")
        .read_dir()
        .expect("Can't read schema dir")
    {
        let entry = entry.expect("Could not read file in schema dir");
        let path = entry.path();

        let s = path.to_str().expect("Invalid UTF-8 in schema path");
        if s.ends_with("/gschemas.compiled") {
            continue;
        }
        println!("cargo:rerun-if-changed={}", s);
    }

    if !code.success() {
        panic!("glib-compile-schemas failed");
    }

    gio::compile_resources(
        "data/resources",
        "data/resources/resources.gresource.xml",
        "compiled.gresource",
    );
}
