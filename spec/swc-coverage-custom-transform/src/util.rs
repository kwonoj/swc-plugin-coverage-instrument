use std::{
    any::type_name,
    panic::{catch_unwind, AssertUnwindSafe},
};

use anyhow::{anyhow, Context, Error};
use napi::Status;
use serde::de::DeserializeOwned;
use swc_core::{
    base::{try_with_handler, HandlerOpts},
    common::{errors::Handler, sync::Lrc, SourceMap, GLOBALS},
};

pub fn try_with<F, Ret>(cm: Lrc<SourceMap>, skip_filename: bool, op: F) -> Result<Ret, Error>
where
    F: FnOnce(&Handler) -> Result<Ret, Error>,
{
    GLOBALS
        .set(&Default::default(), || {
            try_with_handler(
                cm,
                HandlerOpts {
                    skip_filename,
                    ..Default::default()
                },
                |handler| {
                    //
                    let result = catch_unwind(AssertUnwindSafe(|| op(handler)));

                    let p = match result {
                        Ok(v) => return v,
                        Err(v) => v,
                    };

                    if let Some(s) = p.downcast_ref::<String>() {
                        Err(anyhow!("failed to handle: {}", s))
                    } else if let Some(s) = p.downcast_ref::<&str>() {
                        Err(anyhow!("failed to handle: {}", s))
                    } else {
                        Err(anyhow!("failed to handle with unknown panic message"))
                    }
                },
            )
        })
        .map_err(|e| e.to_pretty_error())
}

pub trait MapErr<T>: Into<Result<T, anyhow::Error>> {
    fn convert_err(self) -> napi::Result<T> {
        self.into()
            .map_err(|err| napi::Error::new(Status::GenericFailure, format!("{:?}", err)))
    }
}

impl<T> MapErr<T> for Result<T, anyhow::Error> {}

pub(crate) fn get_deserialized<T, B>(buffer: B) -> napi::Result<T>
where
    T: DeserializeOwned,
    B: AsRef<[u8]>,
{
    let mut deserializer = serde_json::Deserializer::from_slice(buffer.as_ref());
    deserializer.disable_recursion_limit();

    let v = T::deserialize(&mut deserializer)
        .with_context(|| {
            format!(
                "Failed to deserialize buffer as {}\nJSON: {}",
                type_name::<T>(),
                String::from_utf8_lossy(buffer.as_ref())
            )
        })
        .convert_err()?;

    Ok(v)
}

pub(crate) fn deserialize_json<T>(json: &str) -> Result<T, serde_json::Error>
where
    T: DeserializeOwned,
{
    let mut deserializer = serde_json::Deserializer::from_str(json);
    deserializer.disable_recursion_limit();

    T::deserialize(&mut deserializer)
}
