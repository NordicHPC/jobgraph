#![allow(clippy::type_complexity)]

use anyhow::Result;
use chrono::prelude::DateTime;
use chrono::Utc;
use itertools::Itertools;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct Process {
    timestamp: String,
    _hostname: String,
    _num_cores: u64,
    user: String,
    slurm_job_id: String,
    _process: String,
    cpu_percentage: f32,
    mem_kb: u64,
}

use crate::{UsageData, Usage};

// Read data from the csv files and return usage information across time broken down by hostname,
// along with information about the data ranges.

pub fn collect_data(
    data_path: &str,
    job_id: &str,
    dates: &[String],
    hostnames: &[String],
) -> Result<UsageData> {
    let mut data = HashMap::new();

    let now = Utc::now();
    let now_unix_epoch = now.timestamp();

    let mut max_cpu_load = -f64::MAX;
    let mut max_memory_gb = -f64::MAX;

    let mut min_timestamp = i64::MAX;
    let mut max_timestamp = 0;

    let error_message = "ERROR in jobgraph - Can you please email radovan.bast@uit.no and describe what you tried? Thank you!".to_string();

    for hostname in hostnames {
        let mut map: HashMap<i64, (f64, f64)> = HashMap::new();

        for date in dates {
            let (year, month, day) = date.split('-').collect_tuple().expect(&error_message);

            let file_name = format!("{}/{}/{}/{}/{}.csv", data_path, year, month, day, hostname);

            if std::path::Path::new(&file_name).exists() {
                let mut reader = csv::ReaderBuilder::new()
                    .has_headers(false)
                    .from_path(file_name)
                    .expect(&error_message);

                for record in reader.deserialize() {
                    let record: Process = record.expect(&error_message);
                    if record.slurm_job_id == job_id && record.user != "root" {
                        let dt =
                            DateTime::parse_from_rfc3339(&record.timestamp).expect(&error_message);
                        let unix_epoch = dt.timestamp();

                        let cpu_load = (record.cpu_percentage as f64) / 100.0;
                        let mem_gb = (record.mem_kb as f64) / 1024.0 / 1024.0;

                        if map.contains_key(&unix_epoch) {
                            let (mut c, mut m) = map.get(&unix_epoch).unwrap();
                            c += cpu_load;
                            m += mem_gb;
                            map.insert(unix_epoch, (c, m));
                        } else {
                            map.insert(unix_epoch, (cpu_load, mem_gb));
                        }

                        min_timestamp = min_timestamp.min(unix_epoch);
                        max_timestamp = max_timestamp.max(unix_epoch);

                        max_cpu_load = max_cpu_load.max(cpu_load);
                        max_memory_gb = max_memory_gb.max(mem_gb);
                    }
                }
            }
        }

        // convert map k -> c, m to vector of usage data
        let vec = map
            .iter()
            .map(|(k, (c, m))| Usage {
                time: -((now_unix_epoch - *k) as f64) / 3600.0,
                cpu_load: *c,
                mem_gb: *m })
            .collect::<Vec<Usage>>();

        data.insert(hostname.to_string(), vec);
    }

    if min_timestamp == i64::MAX || max_timestamp == 0 {
        anyhow::bail!("ERROR: No data found for this job");
    }

    let min_time_h = -((now_unix_epoch - min_timestamp) as f64) / 3600.0;
    let max_time_h = -((now_unix_epoch - max_timestamp) as f64) / 3600.0;

    Ok(UsageData { min_time_h, max_time_h, max_cpu_load, max_memory_gb, data })
}
