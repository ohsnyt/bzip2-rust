//! CRC computation for the Rust version of the standard BZIP2 library.
//!
//! BZIP2 is a block-oriented approach to compress data.
//!
//! Each block of data has a 32-bit integer which contains the CRC-32 checksum of the uncompressed data
//! contained in the block data. (This is the data BEFORE the RLE1 phase.)
//!
//! All the block CRCs are then combined to create an overall CRC value for the stream.

/// Calculate CRC on the block of data used to build each block that is compressed.
pub fn do_crc(existing_crc: u32, data: &[u8]) -> u32 {
    // For loop is a little faster than .fold
    let mut crc = !existing_crc;
    for b in data {
        crc = (crc << 8) ^ BZ2_CRC32_TABLE[((crc >> 24) ^ (*b as u32)) as usize];
    }
    !crc
}

/// Calculate the stream CRC from each block_crc.
pub fn do_stream_crc(strm_crc: u32, block_crc: u32) -> u32 {
    (strm_crc << 1 | strm_crc >> 31) ^ block_crc
}

// CRC computation for bzip2
/*
The BlockCRC is a 32-bit integer and contains the CRC-32 checksum of the uncompressed data
contained in BlockData.

The calculation is: Start with u32 value of all 1s. For each byte in the input, let the new
crc be the crc shifted left 8 then XORed wiht a lookup from the CRC constant table. The lookup
is based on the crc value shifted right 24 bits and XORed with the byte. Then return the
inverse of the resulting number.

*/

const BZ2_CRC32_TABLE: [u32; 256] = [
    0x00000000u32,
    0x04c11db7u32,
    0x09823b6eu32,
    0x0d4326d9u32,
    0x130476dcu32,
    0x17c56b6bu32,
    0x1a864db2u32,
    0x1e475005u32,
    0x2608edb8u32,
    0x22c9f00fu32,
    0x2f8ad6d6u32,
    0x2b4bcb61u32,
    0x350c9b64u32,
    0x31cd86d3u32,
    0x3c8ea00au32,
    0x384fbdbdu32,
    0x4c11db70u32,
    0x48d0c6c7u32,
    0x4593e01eu32,
    0x4152fda9u32,
    0x5f15adacu32,
    0x5bd4b01bu32,
    0x569796c2u32,
    0x52568b75u32,
    0x6a1936c8u32,
    0x6ed82b7fu32,
    0x639b0da6u32,
    0x675a1011u32,
    0x791d4014u32,
    0x7ddc5da3u32,
    0x709f7b7au32,
    0x745e66cdu32,
    0x9823b6e0u32,
    0x9ce2ab57u32,
    0x91a18d8eu32,
    0x95609039u32,
    0x8b27c03cu32,
    0x8fe6dd8bu32,
    0x82a5fb52u32,
    0x8664e6e5u32,
    0xbe2b5b58u32,
    0xbaea46efu32,
    0xb7a96036u32,
    0xb3687d81u32,
    0xad2f2d84u32,
    0xa9ee3033u32,
    0xa4ad16eau32,
    0xa06c0b5du32,
    0xd4326d90u32,
    0xd0f37027u32,
    0xddb056feu32,
    0xd9714b49u32,
    0xc7361b4cu32,
    0xc3f706fbu32,
    0xceb42022u32,
    0xca753d95u32,
    0xf23a8028u32,
    0xf6fb9d9fu32,
    0xfbb8bb46u32,
    0xff79a6f1u32,
    0xe13ef6f4u32,
    0xe5ffeb43u32,
    0xe8bccd9au32,
    0xec7dd02du32,
    0x34867077u32,
    0x30476dc0u32,
    0x3d044b19u32,
    0x39c556aeu32,
    0x278206abu32,
    0x23431b1cu32,
    0x2e003dc5u32,
    0x2ac12072u32,
    0x128e9dcfu32,
    0x164f8078u32,
    0x1b0ca6a1u32,
    0x1fcdbb16u32,
    0x018aeb13u32,
    0x054bf6a4u32,
    0x0808d07du32,
    0x0cc9cdcau32,
    0x7897ab07u32,
    0x7c56b6b0u32,
    0x71159069u32,
    0x75d48ddeu32,
    0x6b93dddbu32,
    0x6f52c06cu32,
    0x6211e6b5u32,
    0x66d0fb02u32,
    0x5e9f46bfu32,
    0x5a5e5b08u32,
    0x571d7dd1u32,
    0x53dc6066u32,
    0x4d9b3063u32,
    0x495a2dd4u32,
    0x44190b0du32,
    0x40d816bau32,
    0xaca5c697u32,
    0xa864db20u32,
    0xa527fdf9u32,
    0xa1e6e04eu32,
    0xbfa1b04bu32,
    0xbb60adfcu32,
    0xb6238b25u32,
    0xb2e29692u32,
    0x8aad2b2fu32,
    0x8e6c3698u32,
    0x832f1041u32,
    0x87ee0df6u32,
    0x99a95df3u32,
    0x9d684044u32,
    0x902b669du32,
    0x94ea7b2au32,
    0xe0b41de7u32,
    0xe4750050u32,
    0xe9362689u32,
    0xedf73b3eu32,
    0xf3b06b3bu32,
    0xf771768cu32,
    0xfa325055u32,
    0xfef34de2u32,
    0xc6bcf05fu32,
    0xc27dede8u32,
    0xcf3ecb31u32,
    0xcbffd686u32,
    0xd5b88683u32,
    0xd1799b34u32,
    0xdc3abdedu32,
    0xd8fba05au32,
    0x690ce0eeu32,
    0x6dcdfd59u32,
    0x608edb80u32,
    0x644fc637u32,
    0x7a089632u32,
    0x7ec98b85u32,
    0x738aad5cu32,
    0x774bb0ebu32,
    0x4f040d56u32,
    0x4bc510e1u32,
    0x46863638u32,
    0x42472b8fu32,
    0x5c007b8au32,
    0x58c1663du32,
    0x558240e4u32,
    0x51435d53u32,
    0x251d3b9eu32,
    0x21dc2629u32,
    0x2c9f00f0u32,
    0x285e1d47u32,
    0x36194d42u32,
    0x32d850f5u32,
    0x3f9b762cu32,
    0x3b5a6b9bu32,
    0x0315d626u32,
    0x07d4cb91u32,
    0x0a97ed48u32,
    0x0e56f0ffu32,
    0x1011a0fau32,
    0x14d0bd4du32,
    0x19939b94u32,
    0x1d528623u32,
    0xf12f560eu32,
    0xf5ee4bb9u32,
    0xf8ad6d60u32,
    0xfc6c70d7u32,
    0xe22b20d2u32,
    0xe6ea3d65u32,
    0xeba91bbcu32,
    0xef68060bu32,
    0xd727bbb6u32,
    0xd3e6a601u32,
    0xdea580d8u32,
    0xda649d6fu32,
    0xc423cd6au32,
    0xc0e2d0ddu32,
    0xcda1f604u32,
    0xc960ebb3u32,
    0xbd3e8d7eu32,
    0xb9ff90c9u32,
    0xb4bcb610u32,
    0xb07daba7u32,
    0xae3afba2u32,
    0xaafbe615u32,
    0xa7b8c0ccu32,
    0xa379dd7bu32,
    0x9b3660c6u32,
    0x9ff77d71u32,
    0x92b45ba8u32,
    0x9675461fu32,
    0x8832161au32,
    0x8cf30badu32,
    0x81b02d74u32,
    0x857130c3u32,
    0x5d8a9099u32,
    0x594b8d2eu32,
    0x5408abf7u32,
    0x50c9b640u32,
    0x4e8ee645u32,
    0x4a4ffbf2u32,
    0x470cdd2bu32,
    0x43cdc09cu32,
    0x7b827d21u32,
    0x7f436096u32,
    0x7200464fu32,
    0x76c15bf8u32,
    0x68860bfdu32,
    0x6c47164au32,
    0x61043093u32,
    0x65c52d24u32,
    0x119b4be9u32,
    0x155a565eu32,
    0x18197087u32,
    0x1cd86d30u32,
    0x029f3d35u32,
    0x065e2082u32,
    0x0b1d065bu32,
    0x0fdc1becu32,
    0x3793a651u32,
    0x3352bbe6u32,
    0x3e119d3fu32,
    0x3ad08088u32,
    0x2497d08du32,
    0x2056cd3au32,
    0x2d15ebe3u32,
    0x29d4f654u32,
    0xc5a92679u32,
    0xc1683bceu32,
    0xcc2b1d17u32,
    0xc8ea00a0u32,
    0xd6ad50a5u32,
    0xd26c4d12u32,
    0xdf2f6bcbu32,
    0xdbee767cu32,
    0xe3a1cbc1u32,
    0xe760d676u32,
    0xea23f0afu32,
    0xeee2ed18u32,
    0xf0a5bd1du32,
    0xf464a0aau32,
    0xf9278673u32,
    0xfde69bc4u32,
    0x89b8fd09u32,
    0x8d79e0beu32,
    0x803ac667u32,
    0x84fbdbd0u32,
    0x9abc8bd5u32,
    0x9e7d9662u32,
    0x933eb0bbu32,
    0x97ffad0cu32,
    0xafb010b1u32,
    0xab710d06u32,
    0xa6322bdfu32,
    0xa2f33668u32,
    0xbcb4666du32,
    0xb8757bdau32,
    0xb5365d03u32,
    0xb1f740b4u32,
];
