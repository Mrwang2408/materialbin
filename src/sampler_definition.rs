use std::io::Write;

use byteorder::{LittleEndian, WriteBytesExt};
use scroll::{ctx::TryFromCtx, Pread, LE};

use crate::{
    common::{optional_write, read_bool, read_string, write_string},
    MinecraftVersion, WriteError,
};

#[derive(Debug)]
pub struct SamplerDefinition {
    pub register_slot: u16,
    pub access: SamplerAccess,
    pub precision: Precision,
    pub unordered_access: u8,
    pub sampler_type: SamplerType,
    pub texture_format: String,
    pub slot_count: u32,
    pub binding_slot: u8,
    pub unknown_u32: u32,
    pub sampler_state: Option<u8>,
    pub default_texture: Option<String>,
    pub texture_path: Option<String>,
    pub custom_type_info: Option<CustomTypeInfo>,
}
impl<'a> TryFromCtx<'a, MinecraftVersion> for SamplerDefinition {
    type Error = scroll::Error;
    fn try_from_ctx(buffer: &'a [u8], ctx: MinecraftVersion) -> Result<(Self, usize), Self::Error> {
        let mut offset = 0;
        let register_slot: u16 = if ctx == MinecraftVersion::V1_18_30 {
            buffer.gread::<u8>(&mut offset)?.into()
        } else {
            buffer.gread_with(&mut offset, LE)?
        };
        // println!("SamplerAccess");
        let access: SamplerAccess = buffer.gread_with(&mut offset, ())?;       //u8
        let precision: Precision = buffer.gread_with(&mut offset, ())?;        //u8
        let unordered_access: u8 = buffer.gread_with(&mut offset, LE)?;        //u8
        let sampler_type: SamplerType = buffer.gread_with(&mut offset, ctx)?;  //u8
        let texture_format = read_string(buffer, &mut offset)?;
        let slot_count: u32 = buffer.gread_with(&mut offset, LE)?;
        // println!("binding_slot");
        let binding_slot: u8 = if ctx != MinecraftVersion::V1_18_30 {
            buffer.gread_with(&mut offset, LE)?
        } else {
            register_slot.try_into()
                .map_err(|e| scroll::Error::Custom(format!("unknown byte parsing error: {e}")))?
        };
        
        let unknown_u32: u32 = if ctx >= MinecraftVersion::V26_60_22 {
             buffer.gread_with(&mut offset, LE)?
        } else{
            0
        };

        let mut sampler_state = None;
        if ctx >= MinecraftVersion::V1_21_20 {
            if read_bool(buffer, &mut offset)? {
                sampler_state = Some(buffer.gread::<u8>(&mut offset)?);
            }
        }
        let mut default_texture = None;
        let has_default_texture = read_bool(buffer, &mut offset)?;
        if has_default_texture {
            default_texture = Some(read_string(buffer, &mut offset)?);
        }
        // println!("texture_path");
        let mut texture_path = None;
        if ctx >= MinecraftVersion::V1_20_80 {
            let has_texture_path = read_bool(buffer, &mut offset)?;
            if has_texture_path {
                texture_path = Some(read_string(buffer, &mut offset)?);
            }
        }
        let mut custom_type_info: Option<CustomTypeInfo> = None;
        let has_custom_type = read_bool(buffer, &mut offset)?;
        if has_custom_type {
            custom_type_info = Some(buffer.gread_with(&mut offset, ())?)
        }

        Ok((
            Self {
                register_slot,
                access,
                precision,
                unordered_access,
                sampler_type,
                texture_format,
                slot_count,
                binding_slot,
                unknown_u32,
                sampler_state,
                default_texture,
                texture_path,
                custom_type_info,
            },
            offset,
        ))
    }
}
impl SamplerDefinition {
    pub fn write<W>(&self, writer: &mut W, version: MinecraftVersion) -> Result<(), WriteError>
    where
        W: Write,
    {
        if version == MinecraftVersion::V1_18_30 {
            writer.write_u8(self.register_slot.try_into()?)?;
        } else {
            writer.write_u16::<LittleEndian>(self.register_slot)?;
        }
        writer.write_u8(self.access.as_u8())?;
        writer.write_u8(self.precision as u8)?;
        writer.write_u8(self.unordered_access)?;
        writer.write_u8(self.sampler_type.to_u8(version)?)?;
        write_string(&self.texture_format, writer)?;
        writer.write_u32::<LittleEndian>(self.slot_count)?;
        if version != MinecraftVersion::V1_18_30 {
            writer.write_u8(self.binding_slot)?;
        }

        if version >= MinecraftVersion::V26_60_22 {
            writer.write_u32::<LittleEndian>(self.unknown_u32)?;
        }

        if version >= MinecraftVersion::V1_21_20 {
            optional_write(writer, self.sampler_state, |o, v| o.write_u8(v))?;
        }
        optional_write(writer, self.default_texture.as_deref(), |o, v| {
            write_string(v, o)
        })?;
        if version >= MinecraftVersion::V1_20_80 {
            optional_write(writer, self.texture_path.as_deref(), |o, v| {
                write_string(v, o)
            })?;
        }
        optional_write(writer, self.custom_type_info.as_ref(), |o, v| v.write(o))?;

        Ok(())
    }
}
#[derive(Debug)]
pub struct CustomTypeInfo {
    pub name: String,
    pub size: u32,
}
impl<'a> TryFromCtx<'a> for CustomTypeInfo {
    type Error = scroll::Error;
    fn try_from_ctx(buffer: &'a [u8], _: ()) -> Result<(Self, usize), Self::Error> {
        let mut offset = 0;
        let name = read_string(buffer, &mut offset)?;
        let size = buffer.gread_with(&mut offset, LE)?;
        Ok((Self { name, size }, offset))
    }
}
impl CustomTypeInfo {
    pub fn write<W>(&self, writer: &mut W) -> Result<(), WriteError>
    where
        W: Write,
    {
        write_string(&self.name, writer)?;
        writer.write_u32::<LittleEndian>(self.size)?;
        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
pub enum SamplerType {
    Type2D,
    Type2DArray,
    Type2DExternal,
    Type3D,
    TypeCube,
    TypeSamplerCubeArray,
    TypeStructuredBuffer,
    TypeRawBuffer,
    TypeAccelerationStructure,
    Type2DShadow,
    Type2DArrayShadow,
}
impl<'a> TryFromCtx<'a, MinecraftVersion> for SamplerType {
    type Error = scroll::Error;
    fn try_from_ctx(
        buffer: &'a [u8],
        version: MinecraftVersion,
    ) -> Result<(Self, usize), Self::Error> {
        let mut sampler_type: u8 = buffer.pread_with(0, LE)?;
        // On versions before 1.21.20, 5 is Structured Buffer
        // After 1.21.20, 5 is SamplerCubeArray
        // Adjust the difference.
        if version < MinecraftVersion::V1_21_20 && sampler_type >= 5 {
            sampler_type += 1;
        }
        let enum_sub = match sampler_type {
            0 => Self::Type2D,
            1 => Self::Type2DArray,
            2 => Self::Type2DExternal,
            3 => Self::Type3D,
            4 => Self::TypeCube,
            5 => Self::TypeSamplerCubeArray,
            6 => Self::TypeStructuredBuffer,
            7 => Self::TypeRawBuffer,
            8 => Self::TypeAccelerationStructure,
            9 => Self::Type2DShadow,
            10 => Self::Type2DArrayShadow,
            _ => {
                return Err(scroll::Error::Custom(format!(
                    "Invalid sapmler_type: {sampler_type}"
                )))
            }
        };
        Ok((enum_sub, 1))
    }
}
impl SamplerType {
    fn to_u8(self, version: MinecraftVersion) -> Result<u8, WriteError> {
        if version < MinecraftVersion::V1_21_20 {
            return match self {
                Self::TypeSamplerCubeArray => Err(WriteError::Compat("Sampler type is (Sampler Cube Array) ,which is incompatible with versions before 1.21.20".to_string())),
                Self::TypeRawBuffer => Ok(5),
                Self::TypeAccelerationStructure => Ok(6),
                Self::Type2DShadow => Ok(7),
                Self::Type2DArrayShadow => Ok(8),
                _ => Ok(self as u8),
            };
        }
        Ok(self as u8)
    }
}
#[derive(Debug)]
pub enum SamplerAccess {
    None,
    Read,
    Write,
    ReadWrite,
}
#[derive(Debug, Clone, Copy)]
pub enum Precision {
    Low,
    Medium,
    High,
}
impl<'a> TryFromCtx<'a> for Precision {
    type Error = scroll::Error;
    fn try_from_ctx(buffer: &'a [u8], _: ()) -> Result<(Self, usize), Self::Error> {
        let precision: u8 = buffer.pread_with(0, LE)?;
        let precision = match precision {
            0 => Self::Low,
            1 => Self::Medium,
            2 => Self::High,
            _ => {
                return Err(scroll::Error::BadInput {
                    size: 0,
                    msg: "Precision of samplerdef is invalid",
                })
            }
        };
        Ok((precision, 1))
    }
}
impl<'a> TryFromCtx<'a> for SamplerAccess {
    type Error = scroll::Error;
    fn try_from_ctx(buffer: &'a [u8], _: ()) -> Result<(Self, usize), Self::Error> {
        let access: u8 = buffer.pread_with(0, LE)?;
        match access {
            0 => Ok((Self::None, 1)),
            1 => Ok((Self::Read, 1)),
            2 => Ok((Self::Write, 1)),
            3 => Ok((Self::ReadWrite, 1)),
            _ => Err(scroll::Error::Custom(
                "Sampler Access is not valid".to_owned(),
            )),
        }
    }
}
impl SamplerAccess {
    fn as_u8(&self) -> u8 {
        match self {
            Self::None => 0,
            Self::Read => 1,
            Self::Write => 2,
            Self::ReadWrite => 3,
        }
    }
}
