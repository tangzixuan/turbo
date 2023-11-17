//! turborepo-discovery
//!
//! This package contains a number of strategies for discovering various things
//! about a workspace. These traits come with a basic implementation and some
//! adaptors that can be used to compose them together.
//!
//! This powers various intents such as 'query the daemon for this data, or
//! fallback to local discovery if the daemon is not available'. Eventually,
//! these strategies will implement some sort of monad-style composition so that
//! we can track areas of run that are performing sub-optimally.

use tokio_stream::{iter, StreamExt};
use turbopath::AbsoluteSystemPathBuf;

use crate::package_manager::PackageManager;

#[derive(Clone)]
pub struct WorkspaceData {
    pub package_json: AbsoluteSystemPathBuf,
    pub package_manager: PackageManager,
    pub turbo_json: Option<AbsoluteSystemPathBuf>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("discovery unavailable")]
    Unavailable,
    #[error("discovery failed")]
    Failed(String),
}

/// Defines a strategy for discovering packages on the filesystem.
pub trait PackageDiscovery {
    // desugar to assert that the future is Send
    fn discover_packages(
        &mut self,
    ) -> impl std::future::Future<Output = Result<Vec<WorkspaceData>, Error>> + Send;
}

impl<T: PackageDiscovery + Send> PackageDiscovery for Option<T> {
    async fn discover_packages(&mut self) -> Result<Vec<WorkspaceData>, Error> {
        match self {
            Some(d) => d.discover_packages().await,
            None => Err(Error::Unavailable),
        }
    }
}

pub struct LocalPackageDiscovery {
    repo_root: AbsoluteSystemPathBuf,
    package_manager: PackageManager,
}

impl LocalPackageDiscovery {
    pub fn new(repo_root: AbsoluteSystemPathBuf, package_manager: PackageManager) -> Self {
        Self {
            repo_root,
            package_manager,
        }
    }
}

impl PackageDiscovery for LocalPackageDiscovery {
    async fn discover_packages(&mut self) -> Result<Vec<WorkspaceData>, Error> {
        iter(
            self.package_manager
                .get_package_jsons(&self.repo_root)
                .map_err(|e| Error::Failed(e.to_string()))?,
        )
        .then(move |a| {
            let package_manager = self.package_manager.clone();
            async move {
                let potential_turbo = a.parent().expect("non-root").join_component("turbo.json");
                let potential_turbo_exists = tokio::fs::try_exists(potential_turbo.as_path()).await;

                Ok(WorkspaceData {
                    package_manager,
                    package_json: a,
                    turbo_json: potential_turbo_exists
                        .ok()
                        .and_then(|pe| pe.then_some(potential_turbo)),
                })
            }
        })
        .collect()
        .await
    }
}

/// Attempts to run the `primary` strategy for an amount of time
/// specified by `timeout` before falling back to `fallback`
pub struct FallbackPackageDiscovery<P, F> {
    primary: P,
    fallback: F,
    timeout: std::time::Duration,
}

impl<P: PackageDiscovery, F: PackageDiscovery> FallbackPackageDiscovery<P, F> {
    pub fn new(primary: P, fallback: F, timeout: std::time::Duration) -> Self {
        Self {
            primary,
            fallback,
            timeout,
        }
    }
}

impl<A: PackageDiscovery + Send, B: PackageDiscovery + Send> PackageDiscovery
    for FallbackPackageDiscovery<A, B>
{
    async fn discover_packages(&mut self) -> Result<Vec<WorkspaceData>, Error> {
        match tokio::time::timeout(self.timeout, self.primary.discover_packages()).await {
            Ok(Ok(packages)) => Ok(packages),
            Ok(Err(err1)) => match self.fallback.discover_packages().await {
                Ok(packages) => Ok(packages),
                // if the backup is unavailable, return the original error
                Err(Error::Unavailable) => Err(err1),
                Err(err2) => Err(err2),
            },
            Err(_) => self.fallback.discover_packages().await,
        }
    }
}
