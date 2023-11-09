mod bump_timeout;
mod bump_timeout_layer;
mod client;
mod connector;
pub(crate) mod endpoint;
mod server;

pub use client::{DaemonClient, DaemonError};
pub use connector::{DaemonConnector, DaemonConnectorError};
pub use server::{serve, CloseReason, FileWatching};

pub(crate) mod proto {

    tonic::include_proto!("turbodprotocol");

    impl From<PackageManager> for turborepo_repository::package_manager::PackageManager {
        fn from(pm: PackageManager) -> Self {
            match pm {
                PackageManager::Npm => Self::Npm,
                PackageManager::Yarn => Self::Yarn,
                PackageManager::Berry => Self::Berry,
                PackageManager::Pnpm => Self::Pnpm,
                PackageManager::Pnpm6 => Self::Pnpm6,
                PackageManager::Bun => Self::Bun,
            }
        }
    }

    impl From<turborepo_repository::package_manager::PackageManager> for PackageManager {
        fn from(pm: turborepo_repository::package_manager::PackageManager) -> Self {
            match pm {
                turborepo_repository::package_manager::PackageManager::Npm => Self::Npm,
                turborepo_repository::package_manager::PackageManager::Yarn => Self::Yarn,
                turborepo_repository::package_manager::PackageManager::Berry => Self::Berry,
                turborepo_repository::package_manager::PackageManager::Pnpm => Self::Pnpm,
                turborepo_repository::package_manager::PackageManager::Pnpm6 => Self::Pnpm6,
                turborepo_repository::package_manager::PackageManager::Bun => Self::Bun,
            }
        }
    }
}
