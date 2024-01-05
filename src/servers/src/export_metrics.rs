// Copyright 2023 Greptime Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::HashMap;
use std::time::Duration;

use axum::http::HeaderValue;
use common_base::Plugins;
use common_telemetry::metric::{convert_metric_to_write_request, MetricFilter};
use common_telemetry::{error, info};
use common_time::Timestamp;
use hyper::HeaderMap;
use prost::Message;
use reqwest::header::HeaderName;
use serde::{Deserialize, Serialize};
use session::context::QueryContextBuilder;
use snafu::{ensure, ResultExt};
use tokio::time::{self, Interval};

use crate::error::{InvalidExportMetricsConfigSnafu, Result, SendPromRemoteRequestSnafu};
use crate::prom_store::snappy_compress;
use crate::query_handler::PromStoreProtocolHandlerRef;

/// Use to export the metrics generated by greptimedb, encoded to Prometheus [RemoteWrite format](https://prometheus.io/docs/concepts/remote_write_spec/),
/// and send to Prometheus remote-write compatible receiver (e.g. send to `greptimedb` itself)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ExportMetricsOption {
    pub enable: bool,
    #[serde(with = "humantime_serde")]
    pub write_interval: Duration,
    pub self_import: Option<SelfImportOption>,
    pub remote_write: Option<RemoteWriteOption>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct RemoteWriteOption {
    pub url: String,
    pub headers: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct SelfImportOption {
    pub db: String,
}

impl Default for SelfImportOption {
    fn default() -> Self {
        Self {
            db: "information_schema".to_string(),
        }
    }
}

impl Default for ExportMetricsOption {
    fn default() -> Self {
        Self {
            enable: false,
            write_interval: Duration::from_secs(30),
            self_import: None,
            remote_write: None,
        }
    }
}

#[derive(Default, Clone)]
pub struct ExportMetricsTask {
    config: ExportMetricsOption,
    filter: Option<MetricFilter>,
    headers: HeaderMap<HeaderValue>,
    pub send_by_handler: bool,
}

impl ExportMetricsTask {
    pub fn try_new(
        config: &ExportMetricsOption,
        plugins: Option<&Plugins>,
    ) -> Result<Option<Self>> {
        if !config.enable {
            return Ok(None);
        }
        let filter = plugins.map(|p| p.get::<MetricFilter>()).unwrap_or(None);
        ensure!(
            config.write_interval.as_secs() != 0,
            InvalidExportMetricsConfigSnafu {
                msg: "Expected export metrics write_interval greater than zero"
            }
        );
        ensure!(
            (config.remote_write.is_none() && config.self_import.is_some())
                || (config.remote_write.is_some() && config.self_import.is_none()),
            InvalidExportMetricsConfigSnafu {
                msg: "Only one of `self_import` or `remote_write` can be used as the export method"
            }
        );
        if let Some(self_import) = &config.self_import {
            ensure!(
                !self_import.db.is_empty(),
                InvalidExportMetricsConfigSnafu {
                    msg: "Expected `self_import` metrics `db` not empty"
                }
            );
        }
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(remote_write) = &config.remote_write {
            ensure!(
                !remote_write.url.is_empty(),
                InvalidExportMetricsConfigSnafu {
                    msg: "Expected `remote_write` metrics `url` not empty"
                }
            );
            // construct http header
            remote_write.headers.iter().try_for_each(|(k, v)| {
                let header = match TryInto::<HeaderName>::try_into(k) {
                    Ok(header) => header,
                    Err(_) => {
                        return InvalidExportMetricsConfigSnafu {
                            msg: format!("Export metrics: invalid HTTP header name: {}", k),
                        }
                        .fail()
                    }
                };
                match TryInto::<HeaderValue>::try_into(v) {
                    Ok(value) => headers.insert(header, value),
                    Err(_) => {
                        return InvalidExportMetricsConfigSnafu {
                            msg: format!("Export metrics: invalid HTTP header value: {}", v),
                        }
                        .fail()
                    }
                };
                Ok(())
            })?;
        }
        Ok(Some(Self {
            config: config.clone(),
            filter,
            headers,
            send_by_handler: config.self_import.is_some(),
        }))
    }

    pub fn start(&self, handler: Option<PromStoreProtocolHandlerRef>) -> Result<()> {
        if !self.config.enable {
            return Ok(());
        }
        let interval = time::interval(self.config.write_interval);
        let filter = self.filter.clone();
        let _handle = if let Some(self_import) = &self.config.self_import {
            ensure!(
                handler.is_some(),
                InvalidExportMetricsConfigSnafu {
                    msg: "Only `frontend` or `standalone` can use `self_import` as export method."
                }
            );
            common_runtime::spawn_bg(write_system_metric_by_handler(
                self_import.db.clone(),
                handler.unwrap(),
                filter,
                interval,
            ))
        } else if let Some(remote_write) = &self.config.remote_write {
            common_runtime::spawn_bg(write_system_metric_by_network(
                self.headers.clone(),
                remote_write.url.clone(),
                filter,
                interval,
            ))
        } else {
            unreachable!()
        };
        Ok(())
    }
}

/// Send metrics collected by standard Prometheus [RemoteWrite format](https://prometheus.io/docs/concepts/remote_write_spec/)
pub async fn write_system_metric_by_network(
    headers: HeaderMap,
    endpoint: String,
    filter: Option<MetricFilter>,
    mut interval: Interval,
) {
    info!(
        "Start export metrics task to endpoint: {}, interval: {}s",
        endpoint,
        interval.period().as_secs()
    );
    // Pass the first tick. Because the first tick completes immediately.
    interval.tick().await;
    let client = reqwest::Client::new();
    loop {
        interval.tick().await;
        let metric_families = prometheus::gather();
        let request = convert_metric_to_write_request(
            metric_families,
            filter.as_ref(),
            Timestamp::current_millis().value(),
        );
        let resp = match snappy_compress(&request.encode_to_vec()) {
            Ok(body) => client
                .post(endpoint.as_str())
                .header("X-Prometheus-Remote-Write-Version", "0.1.0")
                .header("Content-Type", "application/x-protobuf")
                .headers(headers.clone())
                .body(body)
                .send()
                .await
                .context(SendPromRemoteRequestSnafu),
            Err(e) => Err(e),
        };
        match resp {
            Ok(resp) => {
                if !resp.status().is_success() {
                    error!("report export metrics error, msg: {:#?}", resp);
                }
            }
            Err(e) => error!("report export metrics failed, error {}", e),
        };
    }
}

/// Send metrics collected by our internal handler
/// for case `frontend` and `standalone` dispose it's own metrics,
/// reducing compression and network transmission overhead.
pub async fn write_system_metric_by_handler(
    db: String,
    handler: PromStoreProtocolHandlerRef,
    filter: Option<MetricFilter>,
    mut interval: Interval,
) {
    info!(
        "Start export metrics task by handler, interval: {}s",
        interval.period().as_secs()
    );
    // Pass the first tick. Because the first tick completes immediately.
    interval.tick().await;
    let ctx = QueryContextBuilder::default().current_schema(db).build();
    loop {
        interval.tick().await;
        let metric_families = prometheus::gather();
        let request = convert_metric_to_write_request(
            metric_families,
            filter.as_ref(),
            Timestamp::current_millis().value(),
        );
        if let Err(e) = handler.write(request, ctx.clone()).await {
            error!("report export metrics by handler failed, error {}", e);
        }
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::export_metrics::{
        ExportMetricsOption, ExportMetricsTask, RemoteWriteOption, SelfImportOption,
    };

    #[tokio::test]
    async fn test_config() {
        // zero write_interval
        assert!(ExportMetricsTask::try_new(
            &ExportMetricsOption {
                enable: true,
                write_interval: Duration::from_secs(0),
                ..Default::default()
            },
            None
        )
        .is_err());
        // none self_import and remote_write
        assert!(ExportMetricsTask::try_new(
            &ExportMetricsOption {
                enable: true,
                ..Default::default()
            },
            None
        )
        .is_err());
        // both self_import and remote_write
        assert!(ExportMetricsTask::try_new(
            &ExportMetricsOption {
                enable: true,
                self_import: Some(SelfImportOption::default()),
                remote_write: Some(RemoteWriteOption::default()),
                ..Default::default()
            },
            None
        )
        .is_err());
        // empty db
        assert!(ExportMetricsTask::try_new(
            &ExportMetricsOption {
                enable: true,
                self_import: Some(SelfImportOption { db: "".to_string() }),
                remote_write: None,
                ..Default::default()
            },
            None
        )
        .is_err());
        // empty url
        assert!(ExportMetricsTask::try_new(
            &ExportMetricsOption {
                enable: true,
                self_import: None,
                remote_write: Some(RemoteWriteOption {
                    url: "".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            None
        )
        .is_err());
        // self import but no handle
        let s = ExportMetricsTask::try_new(
            &ExportMetricsOption {
                enable: true,
                self_import: Some(SelfImportOption::default()),
                ..Default::default()
            },
            None,
        )
        .unwrap()
        .unwrap();
        assert!(s.start(None).is_err());
    }
}