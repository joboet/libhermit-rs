use std::path::PathBuf;

use crate::arch::Arch;

xflags::xflags! {
	src "./src/flags.rs"

	/// Run custom build command.
	cmd xtask {
		default cmd help {
			/// Print help information.
			optional -h, --help
		}

		/// Build the kernel.
		cmd build
		{
			/// Build for the architecture.
			required --arch arch: Arch
			/// Directory for all generated artifacts.
			optional --target-dir target_dir: PathBuf
			/// Do not activate the `default` feature.
			optional --no-default-features
			/// Space or comma separated list of features to activate.
			repeated --features features: String
			/// Build artifacts in release mode, with optimizations.
			optional -r, --release
			/// Build artifacts with the specified profile.
			optional --profile profile: String
			/// Enable the `-Z instrument-mcount` flag.
			optional --instrument-mcount
		}

		/// Run clippy for all targets.
		cmd clippy {}
	}
}

// generated start
// The following code is generated by `xflags` macro.
// Run `env UPDATE_XFLAGS=1 cargo build` to regenerate.
#[derive(Debug)]
pub struct Xtask {
	pub subcommand: XtaskCmd,
}

#[derive(Debug)]
pub enum XtaskCmd {
	Help(Help),
	Build(Build),
	Clippy(Clippy),
}

#[derive(Debug)]
pub struct Help {
	pub help: bool,
}

#[derive(Debug)]
pub struct Build {
	pub arch: Arch,
	pub target_dir: Option<PathBuf>,
	pub no_default_features: bool,
	pub features: Vec<String>,
	pub release: bool,
	pub profile: Option<String>,
	pub instrument_mcount: bool,
}

#[derive(Debug)]
pub struct Clippy;

impl Xtask {
	pub const HELP: &'static str = Self::HELP_;

	#[allow(dead_code)]
	pub fn from_env() -> xflags::Result<Self> {
		Self::from_env_()
	}

	#[allow(dead_code)]
	pub fn from_vec(args: Vec<std::ffi::OsString>) -> xflags::Result<Self> {
		Self::from_vec_(args)
	}
}
// generated end
