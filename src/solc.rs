use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Output;
use std::io;

#[derive(Debug, Default)]
pub struct Compiler {
    source_file: PathBuf,

    abi: bool,
    bin: bool,
    overwrite: bool,

    output_dir: PathBuf,
}

impl Compiler {
    pub fn new<P>(source: P) -> Compiler
        where
            P: AsRef<Path>,
    {
        Compiler {
            source_file: source.as_ref().to_owned(),
            output_dir: source.as_ref().parent().unwrap().to_owned(),
            ..Default::default()
        }
    }

    pub fn abi(&mut self) -> &mut Self {
        self.abi = true;
        self
    }

    pub fn bin(&mut self) -> &mut Self {
        self.bin = true;
        self
    }


    pub fn overwrite(&mut self) -> &mut Self {
        self.overwrite = true;
        self
    }

    #[allow(dead_code)]
    fn set_output_dir_path<P>(&mut self, path: P) -> &mut Self
        where
            P: AsRef<Path>,
    {
        self.output_dir = path.as_ref().to_owned();
        self
    }

    pub fn get_output_dir_path(&self) -> PathBuf {
        self.output_dir.clone()
    }

    pub fn compile(&mut self) -> io::Result<Output> {
        let mut cmd = Command::new("solc");

        if self.abi {
            cmd.arg("--abi");
        }

        if self.bin {
            cmd.arg("--bin");
        }

        if self.overwrite {
            cmd.arg("--overwrite");
        }

        cmd.arg("-o");
        cmd.arg(&self.output_dir);

        cmd.arg(&self.source_file);

        println!("cmd: {:?}", cmd);
        cmd.output()
    }
}

