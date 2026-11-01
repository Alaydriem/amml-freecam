//! Writes `i18n/freecam.pot` from `FreecamMessage`. With `--check`, writes nothing and fails when the file on disk
//! is out of date.

use std::path::PathBuf;
use std::process::ExitCode;

use amml_i18n::message::Message;
use amml_i18n_tool::error::ToolError;
use amml_i18n_tool::pot::PotWriter;
use freecam_ui::message::FreecamMessage;

struct FreecamI18nExtract;

impl FreecamI18nExtract {
    fn run() -> ExitCode {
        let check = std::env::args().any(|arg| arg == "--check");
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("i18n")
            .join(format!("{}.pot", FreecamMessage::DOMAIN.name()));
        match PotWriter::write::<FreecamMessage>(&path, check) {
            Ok(true) if check => {
                eprintln!("{}", ToolError::StalePot { path });
                ExitCode::FAILURE
            }
            Ok(_) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        }
    }
}

fn main() -> ExitCode {
    FreecamI18nExtract::run()
}
