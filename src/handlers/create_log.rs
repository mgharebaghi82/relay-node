use std::{env::consts::OS, fs::{self, OpenOptions, File}, io::{BufWriter, Write}};

pub fn write_log(log: String) {
    let mut relaylog_path = "";
    if OS == "windows" {
        relaylog_path = "relaylog.dat"
    } else if OS == "linux" {
        relaylog_path = "/etc/relaylog.dat"
    }
    let log_exist = fs::metadata(relaylog_path).is_ok();
    if log_exist {
        let write_file = OpenOptions::new()
            .append(true)
            .write(true)
            .open(relaylog_path)
            .unwrap();
        let mut writer = BufWriter::new(write_file);
        writeln!(writer, "{}", log).ok();
    } else {
        File::create(relaylog_path).unwrap();
        let write_file = OpenOptions::new()
            .append(true)
            .write(true)
            .open(relaylog_path)
            .unwrap();
        let mut writer = BufWriter::new(write_file);
        writeln!(writer, "{}", log).ok();
    }
}