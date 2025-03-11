use crate::MergeError;
use std::{
    ffi::{OsStr, OsString},
    path::Path,
    process::Command,
    str::FromStr,
};
use tracing::{debug, info};

pub fn create_merged_object(
    merged_path: &Path,
    extra_args: &[&OsStr],
    objects: impl IntoIterator<Item = impl AsRef<Path>>,
    silent: bool,
) -> Result<(), MergeError> {
    let ldflags = if let Ok(ldflags) = std::env::var("ARMERGE_LDFLAGS") {
        ldflags.split(' ').map(OsString::from).collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let ld_path = if let Some(ld_var) = std::env::var_os("LD") {
        ld_var
    } else {
        OsString::from_str("ld").unwrap()
    };
    let mut args = [
        OsString::from("-r"),
        OsString::from("-o"),
        merged_path.as_os_str().to_owned(),
    ]
    .to_vec();
    args.extend(extra_args.iter().map(OsString::from));
    args.extend(ldflags);

    let mut count = 0;
    args.extend(
        objects
            .into_iter()
            .inspect(|_| count += 1)
            .map(|p| p.as_ref().as_os_str().into()),
    );

    let trace_args = args.iter().map(|s| s.to_string_lossy()).collect::<Vec<_>>().join(" ");
    if silent {
        debug!(
            "Merging {} objects: {} {}",
            count,
            &ld_path.to_string_lossy(),
            trace_args
        );
    } else {
        info!(
            "Merging {} objects: {} {}",
            count,
            &ld_path.to_string_lossy(),
            trace_args
        );
    }

    let output = Command::new(&ld_path)
        .args(&args)
        .output()
        .map_err(|e| MergeError::ExternalToolLaunchError { tool: ld_path.to_string_lossy().to_string(), inner: e })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(MergeError::ExternalToolError {
            reason: "Failed to merged object files".to_string(),
            tool: ld_path.to_string_lossy().to_string(),
            args,
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}
