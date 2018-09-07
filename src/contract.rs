use ethabi::Contract;
use std::path::PathBuf;
use ethabi::Token;
use ethabi::Bytes;
use std::fs::File;
use std::path::Path;
use super::Evm;
use std::str;
use rustc_serialize::hex::FromHex;
use solc::Compiler;
use std::io::BufReader;
use std::io::Read;

pub struct CompiledContract {
    bin: PathBuf,
    contract: Contract,
    evm: Evm,
}

impl CompiledContract {
    fn new<T>(output_path: T, source_path: T) -> Self
        where T: AsRef<Path>
    {
        let source_name = source_path.as_ref().file_name().unwrap();
        let abi_path = output_path.as_ref().join(source_name).with_extension("abi");
        let bin_path = abi_path.with_extension("bin");
        CompiledContract {
            bin: bin_path,
            contract: Contract::load(File::open(abi_path).unwrap()).unwrap(),
            evm: Evm::new(),
        }
    }

    fn build_call_data(&self, func: &str, params: &[Token]) -> Bytes {
        self.contract.function(func).map(|function| {
            function.encode_input(params).unwrap()
        }).unwrap()
    }

    pub fn call(&mut self, func: &str, params: &[Token]) {
        let data = self.build_call_data(func, params);
        self.evm.call(&data);
    }

    pub fn deploy(&mut self) {
        let bytes = load_bytes(&self.bin);
        let code = str::from_utf8(&bytes).unwrap();
        let code_bytes = code.from_hex().unwrap();

        self.evm.deploy(&code_bytes);
    }
}

pub fn load_bytes<T>(path: T) -> Vec<u8>
    where T: AsRef<Path>
{
    match File::open(path) {
        Ok(file) => {
            let mut reader = BufReader::new(file);
            let mut contents: Vec<u8> = Vec::new();

            match reader.read_to_end(&mut contents) {
                Ok(_) => contents,
                Err(e) => panic!("Problem reading file {}", e),
            }
        }
        Err(e) => panic!("Could not open file {}", e),
    }
}

pub struct RawContract {
    source_file: PathBuf,
}

impl RawContract {
    pub fn new<T>(source_file: T) -> Self
        where T: AsRef<Path>
    {
        RawContract {
            source_file: source_file.as_ref().to_owned()
        }
    }

    pub fn compile(&self) -> CompiledContract {
        let mut compiler = Compiler::new(&self.source_file);
        let _ = compiler.abi().bin().overwrite().compile().unwrap();
        CompiledContract::new(&compiler.get_output_dir_path(), &self.source_file)
    }
}
