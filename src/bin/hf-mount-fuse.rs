use tracing::{error, info};

use hf_mount::fuse::FuseAdapter;
use hf_mount::setup::setup;

fn main() {
    let s = setup(false);

    let fuse_adapter = FuseAdapter::new(
        s.runtime.handle().clone(),
        s.virtual_fs.clone(),
        s.metadata_ttl,
        s.read_only,
        s.advanced_writes,
    );

    let mut fuse_config = fuser::Config::default();
    fuse_config.mount_options = vec![
        fuser::MountOption::FSName("hf-mount".to_string()),
        fuser::MountOption::DefaultPermissions,
    ];
    if s.read_only {
        fuse_config.mount_options.push(fuser::MountOption::RO);
    }
    fuse_config.acl = fuser::SessionACL::All;
    fuse_config.clone_fd = true;
    fuse_config.n_threads = Some(s.max_threads);

    let session = match fuser::Session::new(fuse_adapter, &s.mount_point, &fuse_config) {
        Ok(s) => s,
        Err(e) => {
            error!("FUSE session failed: {}", e);
            std::process::exit(1);
        }
    };
    let notifier = session.notifier();
    s.virtual_fs.set_invalidator(Box::new(move |ino| {
        if let Err(e) = notifier.inval_inode(fuser::INodeNo(ino), 0, -1) {
            tracing::debug!("inval_inode({}) failed: {}", ino, e);
        }
    }));
    let bg = match session.spawn() {
        Ok(bg) => bg,
        Err(e) => {
            error!("FUSE spawn failed: {}", e);
            std::process::exit(1);
        }
    };
    let _ = bg.join();

    info!("Unmounted cleanly");
}
