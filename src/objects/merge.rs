use anyhow::Result;
use std::ffi::OsString;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::str::FromStr;

pub fn create_merged_object(
    merged_path: &Path,
    extra_args: &[String],
    objects: impl IntoIterator<Item = impl AsRef<Path>>,
    verbose: bool,
) -> Result<()> {
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
    if verbose {
        println!(
            "Merging {} objects: {} {}",
            count,
            &ld_path.to_string_lossy(),
            args.iter()
                .map(|s| s.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ")
        );
    }

    let output = Command::new(&ld_path).args(args).output()?;
    if output.status.success() {
        Ok(())
    } else {
        std::io::stdout().write_all(&output.stdout).unwrap();
        std::io::stderr().write_all(&output.stderr).unwrap();
        panic!(
            "Failed to merged object files with `{}`",
            ld_path.to_string_lossy()
        )
    }
}
