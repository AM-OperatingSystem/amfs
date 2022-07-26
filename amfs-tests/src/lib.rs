#![allow(unknown_lints)]
#![allow(require_stability_comment)]

#[cfg(not(tarpaulin_include))]
pub mod imagegen;

pub fn test_dump(input: String, output: String) {
    let dump_result = std::process::Command::new("/tmp/bin/dumpfs")
        .arg(&input)
        .output()
        .unwrap()
        .stdout;

    std::fs::create_dir_all("dump_result").unwrap();
    std::fs::write(format!("dump_result/{}", output), dump_result).unwrap();

    let result =
        String::from_utf8(std::fs::read(format!("dump_result/{}", output)).unwrap()).unwrap();
    let expected =
        String::from_utf8(std::fs::read(format!("dump_expected/{}", output)).unwrap()).unwrap();

    if result != expected {
        std::process::Command::new("diff")
            .arg("-u")
            .arg(format!("dump_result/{}", output))
            .arg(format!("dump_expected/{}", output))
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
        panic!();
    }
}
