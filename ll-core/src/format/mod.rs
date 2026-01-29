use std::collections::HashMap;
use std::io::Cursor;
use {
    crate::error::{Error, Result},
    serde::{Deserialize, Serialize},
    std::{fmt, path::PathBuf},
};

mod extractors;
mod processors;
use crate::format::extractors::generic_extractor;
pub use extractors::Files;

#[derive(PartialEq,Debug, Clone)]
pub enum Output {
    File(&'static str),
    Folder(&'static str),
}

macro_rules! ecad {
    ([$(($variant:tt, $variant_literal:literal)),*]) => {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub enum ECAD {
            $(
                #[serde(rename = $variant_literal)]
                $variant,
            )*
        }

        impl ::core::convert::TryFrom<&str> for ECAD {
            type Error = Error;

            fn try_from(value: &str) -> ::core::result::Result<Self, Self::Error> {
                match value.to_lowercase().as_str() {
                    $(
                        $variant_literal => Ok(ECAD::$variant),
                    )*
                    _ => Err(Error::EcadNotFound),
                }
            }
        }

        impl ::core::fmt::Display for ECAD {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> ::core::fmt::Result {
                write!(
                    f,
                    "{}",
                    match &self {
                        $(
                            Self::$variant => $variant_literal,
                        )*
                    }
                )
            }
        }

    };
}

ecad!([
    (D3, "3d"),
    (DesignSpark, "designspark"),
    (Eagle, "eagle"),
    (EasyEDA, "easyeda"),
    (KiCad, "kicad"),
    (Zip, "zip")
]);

#[derive(Debug, Clone, PartialEq)]
pub struct Format {
    pub output_path: PathBuf,
    pub name: String,
    pub ecad: ECAD,
    pub create_folder: bool,
    match_path: Vec<&'static str>,
    output: Vec<Output>,
    ignore: Vec<&'static str>,
}

impl Format {
    // Keep the from_ecad constructor used elsewhere in the codebase
    pub fn from_ecad<P: Into<PathBuf>>(name: &String, ecad: ECAD, output_path: P) -> Self {
        let mut fmt = Self {
            output_path: output_path.into(),
            name: (*name).clone(),
            ecad,
            create_folder: false,
            match_path: vec![""],
            output: vec![],
            ignore: vec![],
        };

        match fmt.ecad {
            ECAD::D3 => {
                fmt.create_folder = true;
                fmt.match_path = vec!["3D"];
            }
            ECAD::DesignSpark => {
                fmt.match_path = vec!["DesignSpark PCB"];
            }
            ECAD::Eagle => {
                fmt.match_path = vec!["EAGLE"];
                fmt.ignore = vec!["Readme.html"];
            }
            ECAD::EasyEDA => {
                fmt.match_path = vec!["EasyEDA"];
                fmt.ignore = vec!["Readme.html"];
            }
            ECAD::KiCad => {
                fmt.match_path = vec!["KiCad"];
                // default kicad outputs; consumer code may override
                fmt.output = vec![Output::File("LibraryLoader.lib"), Output::File("LibraryLoader.dcm"), Output::Folder("LibraryLoader.pretty")];
            }
            ECAD::Zip => {
                // no changes
            }
        }

        fmt
    }

    pub fn extract(
        &self,
        archive: &mut zip::ZipArchive<Cursor<&Vec<u8>>>,
    ) -> Result<HashMap<String, Vec<u8>>> {
        Ok(match &self.ecad {
            // * Keep these in alphabetical order
            ECAD::D3 | ECAD::DesignSpark | ECAD::Eagle | ECAD::EasyEDA => {
                generic_extractor(self, archive)?
            }
            ECAD::KiCad => extractors::kicad::extract(self, archive)?,
            ECAD::Zip => unreachable!("ZIP not handled!"),
            // ! NOTE: DO NOT ADD A _ => {} CATCHER HERE!
        })
    }

    pub fn process(&self, output_path: String, output_files: &mut Files, file_path: String, item: &mut Vec<u8>) -> crate::error::Result<()> {
        match &self.ecad {
            ECAD::D3 => processors::d3::process(self, output_path, output_files, file_path, item)?,
            ECAD::DesignSpark => processors::eagle::process(self, output_path, output_files, file_path, item)?,
            ECAD::Eagle => processors::eagle::process(self, output_path, output_files, file_path, item)?,
            ECAD::EasyEDA => processors::easyeda::process(self, output_path, output_files, file_path, item)?,
            ECAD::KiCad => processors::kicad::process(self, output_path, output_files, file_path, item)?,
            ECAD::Zip => unreachable!("ZIP not handled!"),
        };

        Ok(())
    }
}
