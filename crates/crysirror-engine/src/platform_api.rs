// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Public availability query over the same optional backend native VMs use.
use crate::{object::{Class,ClassInfo,MethodInfo,ObjectId,TypeHint as T},EngineContext,Value};
use anyhow::{bail,Context,Result};
use crysirror_core::service::{HostServices,PlatformIntegration};
use std::sync::OnceLock;
pub(crate) struct PlatformClass;
impl Class for PlatformClass {
    fn info(&self)->&ClassInfo {
        static INFO:OnceLock<ClassInfo>=OnceLock::new();
        INFO.get_or_init(||ClassInfo {
            name:"Platform",doc:"Optional host integration backend availability. No backend is unavailable; this is not an entitlement, achievement or SDK-operation result.",properties:vec![],
            methods:vec![MethodInfo { name:"integration",params:vec![("name",T::String)],returns:T::Map,
                doc:"Read owner-scoped backend availability for steam. Returns client/name/available/backend; backend null means no SDK backend is installed. Unknown integration names fail." }],
        })
    }
    fn get(&self,_:&mut EngineContext,_:ObjectId,_:&str)->Result<Value> {bail!("Platform has no properties")}
    fn call(&self,cx:&mut EngineContext,_:ObjectId,name:&str,args:&[Value])->Result<Value> {
        if name!="integration" {bail!("unknown Platform method {name}")}
        let key=args.first().context("platform integration name")?.as_str()?;
        let integration=match key {"steam"=>PlatformIntegration::Steam,_=>bail!("unknown platform integration {key}")};
        let state=cx.state()?;
        let info=state.runtime.host.platform_integration(state.client,integration)?;
        Ok(Value::from_json(serde_json::json!({"client":info.client.0,"name":key,"available":info.available,"backend":info.backend})))
    }
}
