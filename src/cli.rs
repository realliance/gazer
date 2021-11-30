//! Handles utility command line arguments

use clap::{App, Arg, ArgMatches};

const GEN_CRD_ARG: &str = "gencrd";

/// Command Line argument flags
pub struct CliArgs {
  pub gen_crd: bool,
}

impl<'a> From<ArgMatches<'a>> for CliArgs {
  fn from(args: ArgMatches) -> Self {
    CliArgs {
      gen_crd: args.is_present(GEN_CRD_ARG),
    }
  }
}

/// Handles command line arguments via clap.
pub fn build_cli() -> CliArgs {
  let matches = App::new("Gazer")
    .arg(Arg::with_name(GEN_CRD_ARG).long("crd"))
    .get_matches();

  CliArgs::from(matches)
}
