use serde_json::Value;

use chrono::Utc;
use std::process::{Command, Stdio};

use crate::dates;
use crate::nodelist;

pub fn get_hostnames(out: &str) -> Vec<String> {
    let v: Value = serde_json::from_str(out).unwrap();

    let s = format!("{}", &v["jobs"][0]["nodes"]).replace('\"', "");
    nodelist::parse_list(&s)
}

pub fn get_dates(out: &str) -> Vec<String> {
    let v: Value = serde_json::from_str(out).unwrap();

    let start = &v["jobs"][0]["time"]["start"];
    let end = &v["jobs"][0]["time"]["end"];

    let start_epoch = start.as_i64().expect("Failed to parse start date");
    let mut end_epoch = end.as_i64().expect("Failed to parse end date");

    if end_epoch == 0 {
        let now = Utc::now();
        end_epoch = now.timestamp();
    }

    dates::date_range(start_epoch, end_epoch)
}

pub fn job_id_is_valid(out: &str, err: &str) -> bool {
    if err.contains("sacct: fatal") {
        return false;
    }

    let v: Value = serde_json::from_str(out).unwrap();

    if &v["errors"][0]["description"] == "Nothing found" {
        return false;
    }

    true
}

pub fn job_id_is_array(job_id: &str, out: &str) -> bool {
    if job_id.contains('_') {
        return false;
    }

    let v: Value = serde_json::from_str(out).unwrap();

    let job_id_number: usize = job_id.parse().expect("Failed to parse job ID");
    if v["jobs"][0]["array"]["job_id"] == job_id_number {
        return true;
    }

    false
}

pub fn array_subjob_id(out: &str) -> String {
    let v: Value = serde_json::from_str(out).unwrap();

    let job_id = &v["jobs"][0]["job_id"];
    let job_id_number: usize = job_id.as_i64().expect("Failed to parse job ID") as usize;

    format!("{}", job_id_number)
}

pub fn requested_num_cores(out: &str) -> usize {
    let v: Value = serde_json::from_str(out).unwrap();

    let n = &v["jobs"][0]["required"]["CPUs"];
    n.as_i64().expect("Failed to parse required CPUs") as usize
}

pub fn requested_memory(out: &str) -> usize {
    let v: Value = serde_json::from_str(out).unwrap();

    let n = &v["jobs"][0]["required"]["memory"];
    n.as_i64().expect("Failed to parse required memory") as usize
}

pub fn sacct(job_id: &str, debug: bool) -> (String, String) {
    let mut command = "sacct";
    let mut args = vec!["-j", job_id, "--json"];

    if debug {
        command = "cat";
        args = vec!["debug/json"];
    }

    let output = Command::new(command)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("ERROR in jobgraph - Can you please email radovan.bast@uit.no and describe what you tried? Thank you!");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    (stdout.to_string(), stderr.to_string())
}
