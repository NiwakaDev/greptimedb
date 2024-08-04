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

use async_trait::async_trait;
use common_procedure::error::{Result as ProcedureResult, ToJsonSnafu};
use common_procedure::{Context, LockKey, Procedure, Status};
use common_telemetry::{info, warn};
use serde::{Deserialize, Serialize};
use snafu::{ensure, ResultExt};

use super::utils::handle_retry_error;
use crate::ddl::DdlContext;
use crate::error::{self, Result};
use crate::key::schema_name::{SchemaName, SchemaNameKey};
use crate::lock_key::{CatalogLock, SchemaLock};
use crate::rpc::ddl::AlterDatabaseTask;
use crate::ClusterId;

pub struct AlterDatabaseProcedure {
    context: DdlContext,
    data: AlterDatabaseData,
}

impl AlterDatabaseProcedure {
    const TYPE_NAME: &'static str = "metasrv-procedure::AlterDatabase";

    pub fn new(_cluster_id: ClusterId, task: AlterDatabaseTask, context: DdlContext) -> Self {
        AlterDatabaseProcedure {
            context,
            data: AlterDatabaseData {
                state: AlterDatabaseState::Prepare,
                task,
            },
        }
    }

    async fn on_prepare(&mut self) -> Result<Status> {
        self.data.state = AlterDatabaseState::UpdateMetadata;
        let exist = self
            .context
            .table_metadata_manager
            .schema_manager()
            .exists(SchemaNameKey {
                catalog: &self.data.task.alter_database.catalog_name,
                schema: &self.data.task.alter_database.schema_name,
            })
            .await?;
        ensure!(
            exist,
            error::SchemaNotFoundSnafu {
                table_schema: self.data.task.alter_database.schema_name.clone()
            }
        );
        Ok(Status::Executing { persist: true })
    }

    async fn on_update_metadata(&mut self) -> Result<Status> {
        self.context
            .table_metadata_manager
            .update_schema_info()
            .await?;
        Ok(Status::done())
    }
}

#[async_trait]
impl Procedure for AlterDatabaseProcedure {
    fn type_name(&self) -> &str {
        Self::TYPE_NAME
    }
    async fn execute(&mut self, _ctx: &Context) -> ProcedureResult<Status> {
        let state = &self.data.state;
        match state {
            AlterDatabaseState::Prepare => self.on_prepare().await,
            AlterDatabaseState::UpdateMetadata => self.on_update_metadata().await,
        }
        .map_err(handle_retry_error)
    }
    fn lock_key(&self) -> LockKey {
        let lock_key = vec![
            CatalogLock::Read(&self.data.task.alter_database.catalog_name).into(),
            SchemaLock::write(
                &self.data.task.alter_database.catalog_name,
                &self.data.task.alter_database.schema_name,
            )
            .into(),
        ];
        LockKey::new(lock_key)
    }
    fn dump(&self) -> common_procedure::Result<String> {
        serde_json::to_string(&self.data).context(ToJsonSnafu)
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum AlterDatabaseState {
    Prepare,
    UpdateMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
struct AlterDatabaseData {
    state: AlterDatabaseState,
    task: AlterDatabaseTask,
}
