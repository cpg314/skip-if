use std::path::Path;

use skip_if::skip_if;

fn retriable(e: &anyhow::Error) -> bool {
    e.to_string().contains("retry")
}


#[tokio::test]
async fn async_fn() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    #[skip_if(output = "folder.join(id)", strategy = "skip_if::FileExists")]
    async fn to_cache(id: &str, folder: &Path) -> Result<(), anyhow::Error> {
        tokio::time::sleep(std::time::Duration::from_nanos(1)).await;
        Ok(())
    }
    let dir = tempfile::tempdir()?;
    to_cache("0", dir.path()).await?;
    Ok(())
}

#[test]
fn file_exists() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    #[skip_if(output = "folder.join(id)", strategy = "skip_if::FileExists")]
    fn to_cache(id: &str, run: bool, folder: &Path) -> Result<(), anyhow::Error> {
        assert!(run);
        std::fs::create_dir_all(folder)?;
        assert_eq!(skip_if_output, &folder.join(id));
        std::fs::write(skip_if_output, id.to_string())?;
        Ok(())
    }
    let dir = tempfile::tempdir()?;
    let output = dir.path().join("test");
    to_cache("0", true, &output)?;
    to_cache("0", false, &output)?;
    Ok(())
}

#[test]
fn markers() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    #[skip_if(
        output = "output",
        strategy = "skip_if::Markers::default().retriable(retriable)",
        args_skip = "fail,run"
    )]
    fn to_cache(fail: Fail, run: bool, id: u64, output: &Path) -> anyhow::Result<()> {
        assert!(run);
        fail.run()?;
        std::fs::write(output, "")?;
        Ok(())
    }
    markers_tests(to_cache, false)?;

    Ok(())
}

enum Fail {
    No,
    Unretriable,
    Retriable,
}
impl Fail {
    fn run(&self) -> anyhow::Result<()> {
        match self {
            Fail::No => Ok(()),
            Fail::Unretriable => anyhow::bail!(""),
            Fail::Retriable => anyhow::bail!("retry"),
        }
    }
}

#[test]
fn markers_folder() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    #[skip_if(
        output = "output",
        strategy = "skip_if::Markers::default().folder().retriable(retriable)",
        args_skip = "fail,run"
    )]
    fn to_cache(fail: Fail, run: bool, id: u64, output: &Path) -> anyhow::Result<()> {
        assert!(run);
        fail.run()?;
        std::fs::create_dir_all(output)?;
        Ok(())
    }
    markers_tests(to_cache, true)?;
    Ok(())
}

fn markers_tests(
    to_cache: impl Fn(Fail, bool, u64, &Path) -> anyhow::Result<()>,
    folder: bool,
) -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;

    // Success
    let output = dir.path().join("test");
    for run in [true, false] {
        to_cache(Fail::No, run, 0, &output)?;
    }
    if folder {
        assert!(output.join("success").exists());
    } else {
        assert!(dir.path().join("test.success").exists());
    }
    // Argument change
    to_cache(Fail::No, true, 1, &output)?;

    // Failure
    let output = dir.path().join("test2");
    let _ = to_cache(Fail::Unretriable, true, 0, &output);
    if folder {
        assert!(output.join("failure").exists());
    } else {
        assert!(dir.path().join("test2.failure").exists());
    }
    to_cache(Fail::Unretriable, false, 0, &output)?;
    // Argument change
    to_cache(Fail::No, true, 1, &output)?;
    if folder {
        assert!(output.join("success").exists());
    } else {
        assert!(dir.path().join("test2.success").exists());
    }

    // Failure with retry
    let _ = to_cache(Fail::Retriable, true, 0, &output);
    to_cache(Fail::No, true, 0, &output)?;

    Ok(())
}
