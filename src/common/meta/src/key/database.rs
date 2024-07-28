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

use crate::error::{self, Result};
use crate::key::SchemaManager;
use crate::kv_backend::KvBackendRef;

struct SchemaInfoManager {}

pub struct SchemaMetadataManager {
    kv_backend: KvBackendRef,
    // maybe rename SchemaManager to SchemaNameManager?
    schema_name_manager: SchemaManager,
    schema_info_manager: SchemaInfoManager,
}

impl SchemaMetadataManager {
    /// Creates the schema metadata
    async fn create_schema_metadata() -> Result<()> {
        /***
        Create the schema name and options like ttl.
        let mut txn = Txn::merge_all(vec![
            create_schema_name_txn,
            create_schema_info_txn,
        ]);
        // let response = self.kv_backend.txn(txn).await?;
        ***/
        todo!()
    }

    /// Deletes the schema metadata
    async fn delete_schema_metadata(
        &self,
        flow_id: FlowId,
        flow_value: &FlowInfoValue,
    ) -> Result<()> {
        todo!();
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_create_schema_metadata() {}

    #[tokio::test]
    async fn test_delete_schema_metadata() {}
}
