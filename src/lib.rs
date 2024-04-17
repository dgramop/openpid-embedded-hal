use indoc::formatdoc;
use openpid::prelude::*;
use openpid::{CodegenError, config::OpenPID};
use std::fs;

const TAB: &str = "    ";

#[derive(Debug)]
pub struct RustEmbeddedHal<'a> {
    pid: &'a OpenPID,
    target: std::path::PathBuf
}

#[derive(Debug, Clone)]
struct Var {
    name: String,
    datatype: String,
    desc: Option<String>
}

impl Var {
    fn new(name: impl Into<String>, datatype: impl Into<String>, doc: Option<impl Into<String>>) -> Var {
        Var {
            name: name.into(),
            datatype: datatype.into(),
            desc: match doc {
                Some(s) => Some(s.into()),
                None => None
            }
        }
    }
}

#[derive(Debug)]
struct CodeChunk {
    /// Generated Program Code
    data: String,

    /// Variables referenced by `data`. For debugging codegen and analysis of data flow
    inputs: Vec<Var>,

    /// Variables referenced by `data`. For debugging codegen and analysis of data flow
    outputs: Vec<Var>,
}

impl<'a> RustEmbeddedHal<'a> {
    pub fn new(pid: &'a OpenPID, target: impl Into<std::path::PathBuf>) -> Self {
        Self {
            pid,
            target: target.into()
        }
    }

    /// Initializes Cargo Package at the given path
    fn cargo_init(&self) -> Result<(), std::io::Error> {
        fs::create_dir_all(&self.target)?;
        fs::create_dir_all(&self.target.join("src"))?;
        fs::write(self.target.join(".gitignore"), formatdoc! {"
        target/
        Cargo.lock
        **/*.rs.bk
        debug/
        *.pdb
        "})?;

        let version = match &self.pid.doc_version {
            Some(version) => {
                version
            },
            None => {
                eprintln!("No document version provided. This may result in problems with cargo publish later on. This behavior is deprecated and will abort codegen in the future. Defaulting codegen'd crate version 0.1.0");
                "0.1.0"
            }
        };

        //important: escape user-contributed strings for toml, just in general a good idea. This is
        //an excellent attack vector for an openpid provider
        fs::write(&self.target.join("Cargo.toml"), formatdoc!("
        [package]
        name = \"{name}\"
        version = \"{version}\"
        edition = \"2021\"
        authors = [\"OpenPID Codegen\"]
        description = \"{desc}\"
        categories = [\"embedded\", \"no-std\", \"parser-implementations\", \"hardware-support\"]
        keywords = [\"driver\", \"openpid\"]

        [dependencies]
        embedded_hal = \"1\"

        # For serial/UART stream representation
        embedded-io = \"0.6.1\"
        ", 
        name = self.pid.device_info.name.escape_default(),
        version = version.escape_default(),
        desc = self.pid.device_info.description.escape_default()
        ))?;

        let src_dir = self.target.join("src");

        fs::write(src_dir.join("lib.rs"), formatdoc!("
        #[no_std]

        extern crate embedded_hal;
        extern crate embedded_io;

        struct BitStream {{
        {TAB}/// Underlying byte stream
        {TAB}stream: todo!(),

        {TAB}/// Stores bits if a non-byte-aligned read/write occurs, for future reading/writing
        {TAB}leftover: u8
        }}

        /// Reads the given type from the bit stream
        pub trait<Underlying, const S: PacketSegment> Get<T> {{
        {TAB}fn get(stream: BitStream) -> T;
        }}

        /// Writes the given type to the bit stream
        pub trait<Underlying, const S: PacketSegment> Put<Underlying, const ET: EmbeddedType> {{
        {TAB}/// Returns the number of bits written
        {TAB}fn put(steam: BitStream) -> usize;
        }}

        "))?;

        todo!()
    }

    /// Codegens a payload for tx
    fn codegen_out_payload(&self, name: &str, load: Payload) -> Result<CodeChunk, CodegenError> { // rust UART i2c
        let mut inputs = Vec::<Var>::new();
        let outputs = Vec::<Var>::new();

        let mut seg_writes = String::new();
        for segment in load.segments {
            let out_seg = self.codegen_out_segment(name, &segment, None)?;
            seg_writes.push_str(TAB);
            let indented_seg_write = out_seg.data.replace("\n", &format!("\n{}", TAB));
            seg_writes.push_str(&indented_seg_write);
            inputs.extend(out_seg.inputs.into_iter());

        }
        
        let mut input_docs = String::new();
        for input in &inputs {
            let description = match &input.desc {
                Some(desc) => desc.split("\n").map(|dl| dl.escape_default().to_string()).collect::<Vec<String>>().join("\n///  "),
                None => continue
            }; //refactor to map

            input_docs.push_str(&format!("/// * `{name}` - {desc}", name = input.name, desc = description))
        }

        let docs = formatdoc!(
            "
            /// {payload_desc}
            {input_docs}
            ",
            payload_desc = load.description.replace("\n", "\n///")

            );

        let code = formatdoc!(
            "
            /// {docs}
            fn {payload_name}({args}) {{
            {seg_writes}
            }}
            ", 
            payload_name = name.escape_default(),
            args = inputs.iter().map(|v| format!("{name}: {datatype}",
                                                 name = v.name.escape_default(),
                                                 datatype = v.datatype
                                                 )
                                     ).collect::<Vec<_>>().join(", ")
            );

        Ok(CodeChunk { data: code, inputs, outputs })
    }

    fn codegen_struct(&self, struct_name: &str, rs: &ReusableStruct) -> Result<CodeChunk, CodegenError> {
        let fields = Vec::new();

        let desc = match &rs.description {
            Some(desc) => format!("///{}", desc.split("\n").map(|f| f.escape_default().to_string()).collect::<Vec<String>>().join("\n///")),
            None => "".to_owned()
        };

        let code = formatdoc!(
            "
            {desc}
            struct {struct_name} {{
            {TAB}//TODO: vars
            }}
            ",
            struct_name = struct_name.escape_default()
            );

        todo!();
        Ok(CodeChunk { data: code, inputs: fields.clone(), outputs: fields })
    }

    /// Codegens a PacketSegment for transmission
    ///
    /// # Arguments
    /// * `source_name` - Name of the payload or struct this segment is a part of
    /// * `seg` - The packet segment to generate a write for
    /// * `struct_config` - If this segment came from a Struct, informs codegen of how to generate
    /// the given writes
    fn codegen_out_segment(&self, payload_name: &str, seg: &PacketSegment, struct_config: Option<&ReusableStruct>) -> Result<CodeChunk, CodegenError> {

        let mut inputs = Vec::<Var>::new();
        let outputs = Vec::<Var>::new();
        let mut code = String::new(); 

        let spacing = TAB;

        // tells the rest of the code to prefix input variables with a given prefix, for example 
        // if the data we're interested in is contained within a struct instance
        let prefix = match struct_config {
            Some(ReusableStruct {name, fields:_, description}) => {
                // since we are codegenning for the contents of an input struct, we depend on that
                // struct
                inputs.push(Var {
                    name: name.clone(),
                    datatype: name.to_owned(), //TODO: to camel case? need a way to convert rs_name
                                               //into a rust name
                    desc: description.to_owned()
                });

                format!("{name}.")
            },
            None => "".to_owned()
        };

        match seg {
            PacketSegment::Sized { name, bits, datatype, description } => {

                //TODO: actual read/write interface
                match datatype {
                    SizedDataType::Raw => {
                        let var = Var::new(format!("{prefix}{name}"), format!("&[u8; {}]", if bits%8 == 0 { bits/8 } else { bits/8 + 1 } ), description.as_ref());
                        code.push_str(&format!("{spacing}write({var})\n", 
                                      var = &var.name
                                      ));
                        inputs.push(var);
                    },
                    SizedDataType::Const { data } => {
                        if *bits as usize != data.len() * 8 {
                            unimplemented!("Not integral number of bytes for const values not yet supported");
                        }

                        code.push_str(&formatdoc!(
                        "
                        {spacing}// {name}
                        {spacing}write([{data}]);
                        ", 
                        data = data.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(", "),
                        name = name.escape_default()
                        ))
                    },
                    SizedDataType::Integer { endianness, signing } => {
                        if *bits != 8 && *bits != 16 && *bits != 32 && *bits != 64 {
                            unimplemented!("Codegen for rust don't yet handle non-standard length integers.")
                        }
                        
                        let var = Var::new(format!("{prefix}{name}"), format!("{}{}", match signing {
                            Signing::Unsigned => "u",
                            Signing::TwosComplement => "i",
                            Signing::OnesComplement => unimplemented!("One's complement not yet implemented")
                        }, bits), description.as_ref());


                        // to get one's complement, take the absolute value, and set the highest
                        // bit if it was negative
                        code.push_str(&formatdoc!(
                                "
                                {spacing}write({var}.{bytes_function}())
                                ",
                                var = &var.name,
                                bytes_function = match endianness {
                                    Endianness::BigEndian => "to_be_bytes",
                                    Endianness::LittleEndian => "to_le_bytes",
                                }
                                ));

                        inputs.push(var);
                    },
                    SizedDataType::StringUTF8 => {
                        let var = Var::new(format!("{prefix}{name}"), format!("&str"), description.as_ref());
                        //TODO: enforce size constraint in generated code, what if someone sends a
                        //too-big or too-small &str? ideally at comptime

                        // no partial string writes
                        assert!(bits % 8 == 0);

                        code.push_str(&formatdoc!(
                                "
                                {spacing}write({var}.as_bytes())
                                ",
                                var = var.name));

                        inputs.push(var);
                    },
                    SizedDataType::FloatIEEE { endianness } => {
                        if *bits != 32 && *bits != 64 {
                            unimplemented!("only IEEE 32 and 64 bit floats currently supported")
                        }
                        let var = Var::new(name, format!("f{}",bits), description.as_ref());

                        code.push_str(&formatdoc!(
                                "
                                {spacing}write({var}.{bytes_function}())
                                ",
                                var = var.name,
                                bytes_function = match endianness {
                                    Endianness::BigEndian => "to_be_bytes",
                                    Endianness::LittleEndian => "to_le_bytes",
                                }
                                ));

                        inputs.push(var);
                    }
                }
            },
            PacketSegment::Unsized { name, datatype, termination, description } => {
                match datatype {
                    UnsizedDataType::Array { item_struct } => {
                    },
                    UnsizedDataType::StringUTF8 => {
                    },
                    UnsizedDataType::Raw => {
                    }
                }
                unimplemented!("Unsized data not yet supported")
            }
            PacketSegment::Struct { name, struct_name } => {
                let reusable_struct = match self.pid.structs.get(name) {
                    Some(rs) => rs,
                    None => {
                        return Err(CodegenError::NoStruct { wanted_by_payload: payload_name.to_string(), wanted_by_field: name.clone(), struct_name: struct_name.clone() });
                    }
                };
                //TODO: add struct as input!
                // lookup the struct and recurse
                unimplemented!("Struct referencing not yet supported")
            }
        };

        Ok(CodeChunk { data: code, inputs, outputs })
    }
}

impl<'a> Codegen for RustEmbeddedHal<'a> {
    fn codegen(&mut self) -> Result<(), CodegenError> {
        self.cargo_init();

        Ok(())
    }
}
