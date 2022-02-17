//! Stake Farms

use {
    crate::{pack::*, string::ArrayString64, traits::*},
    arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs},
    num_enum::TryFromPrimitive,
    serde::{Deserialize, Serialize},
    serde_json::to_string,
    solana_program::{program_error::ProgramError, pubkey::Pubkey},
};

#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub enum FarmRoute {
    Raydium {
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        farm_id: Pubkey,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        farm_authority: Pubkey,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        farm_lp_token_account: Pubkey,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        farm_reward_token_a_account: Pubkey,
        #[serde(
            deserialize_with = "optional_pubkey_deserialize",
            serialize_with = "optional_pubkey_serialize"
        )]
        farm_reward_token_b_account: Option<Pubkey>,
    },
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum FarmRouteType {
    Raydium,
    Saber,
    Orca,
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum FarmType {
    SingleReward,
    DualReward,
    ProtocolTokenStake,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub struct Farm {
    #[serde(
        serialize_with = "as64_serialize",
        deserialize_with = "as64_deserialize"
    )]
    pub name: ArrayString64,
    pub version: u16,
    pub farm_type: FarmType,
    pub official: bool,
    pub refdb_index: Option<u32>,
    pub refdb_counter: u16,
    #[serde(
        deserialize_with = "optional_pubkey_deserialize",
        serialize_with = "optional_pubkey_serialize"
    )]
    pub lp_token_ref: Option<Pubkey>,
    #[serde(
        deserialize_with = "optional_pubkey_deserialize",
        serialize_with = "optional_pubkey_serialize"
    )]
    pub reward_token_a_ref: Option<Pubkey>,
    #[serde(
        deserialize_with = "optional_pubkey_deserialize",
        serialize_with = "optional_pubkey_serialize"
    )]
    pub reward_token_b_ref: Option<Pubkey>,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub router_program_id: Pubkey,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub farm_program_id: Pubkey,
    pub route: FarmRoute,
}

impl Named for Farm {
    fn name(&self) -> ArrayString64 {
        self.name
    }
}

impl Versioned for Farm {
    fn version(&self) -> u16 {
        self.version
    }
}

impl Farm {
    pub const MAX_LEN: usize = 655;
    pub const RAYDIUM_FARM_LEN: usize = 400;
    pub const SABER_FARM_LEN: usize = 655;
    pub const ORCA_FARM_LEN: usize = 399;

    pub fn get_size(&self) -> usize {
        match self.route {
            FarmRoute::Raydium { .. } => Farm::RAYDIUM_FARM_LEN,
            FarmRoute::Saber { .. } => Farm::SABER_FARM_LEN,
            FarmRoute::Orca { .. } => Farm::ORCA_FARM_LEN,
        }
    }

    pub fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        match self.route {
            FarmRoute::Raydium { .. } => self.pack_raydium(output),
            FarmRoute::Saber { .. } => self.pack_saber(output),
            FarmRoute::Orca { .. } => self.pack_orca(output),
        }
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; Farm::MAX_LEN] = [0; Farm::MAX_LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidAccountData)
        }
    }

    pub fn unpack(input: &[u8]) -> Result<Farm, ProgramError> {
        check_data_len(input, 1)?;
        let farm_route_type = FarmRouteType::try_from_primitive(input[0])
            .or(Err(ProgramError::InvalidAccountData))?;
        match farm_route_type {
            FarmRouteType::Raydium => Farm::unpack_raydium(input),
            FarmRouteType::Saber => Farm::unpack_saber(input),
            FarmRouteType::Orca => Farm::unpack_orca(input),
        }
    }

    fn pack_raydium(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, Farm::RAYDIUM_FARM_LEN)?;

        if let FarmRoute::Raydium {
            farm_id,
            farm_authority,
            farm_lp_token_account,
            farm_reward_token_a_account,
            farm_reward_token_b_account,
        } = self.route
        {
            let output = array_mut_ref![output, 0, Farm::RAYDIUM_FARM_LEN];

            let (
                farm_route_type_out,
                name_out,
                version_out,
                farm_type_out,
                official_out,
                refdb_index_out,
                refdb_counter_out,
                lp_token_ref_out,
                reward_token_a_ref_out,
                reward_token_b_ref_out,
                router_program_id_out,
                farm_program_id_out,
                farm_id_out,
                farm_authority_out,
                farm_lp_token_account_out,
                farm_reward_token_a_account_out,
                farm_reward_token_b_account_out,
            ) = mut_array_refs![
                output, 1, 64, 2, 1, 1, 5, 2, 33, 33, 33, 32, 32, 32, 32, 32, 32, 33
            ];

            farm_route_type_out[0] = FarmRouteType::Raydium as u8;

            pack_array_string64(&self.name, name_out);
            *version_out = self.version.to_le_bytes();
            farm_type_out[0] = self.farm_type as u8;
            official_out[0] = self.official as u8;
            pack_option_u32(self.refdb_index, refdb_index_out);
            *refdb_counter_out = self.refdb_counter.to_le_bytes();
            pack_option_key(&self.lp_token_ref, lp_token_ref_out);
            pack_option_key(&self.reward_token_a_ref, reward_token_a_ref_out);
            pack_option_key(&self.reward_token_b_ref, reward_token_b_ref_out);
            router_program_id_out.copy_from_slice(self.router_program_id.as_ref());
            farm_program_id_out.copy_from_slice(self.farm_program_id.as_ref());
            farm_id_out.copy_from_slice(farm_id.as_ref());
            farm_authority_out.copy_from_slice(farm_authority.as_ref());
            farm_lp_token_account_out.copy_from_slice(farm_lp_token_account.as_ref());
            farm_reward_token_a_account_out.copy_from_slice(farm_reward_token_a_account.as_ref());
            pack_option_key(
                &farm_reward_token_b_account,
                farm_reward_token_b_account_out,
            );

            Ok(Farm::RAYDIUM_FARM_LEN)
        } else {
            Err(ProgramError::InvalidAccountData)
        }
    }

    fn unpack_raydium(input: &[u8]) -> Result<Farm, ProgramError> {
        check_data_len(input, Farm::RAYDIUM_FARM_LEN)?;

        let input = array_ref![input, 1, Farm::RAYDIUM_FARM_LEN - 1];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            name,
            version,
            farm_type,
            official,
            refdb_index,
            refdb_counter,
            lp_token_ref,
            reward_token_a_ref,
            reward_token_b_ref,
            router_program_id,
            farm_program_id,
            farm_id,
            farm_authority,
            farm_lp_token_account,
            farm_reward_token_a_account,
            farm_reward_token_b_account,
        ) = array_refs![input, 64, 2, 1, 1, 5, 2, 33, 33, 33, 32, 32, 32, 32, 32, 32, 33];

        Ok(Self {
            name: unpack_array_string64(name)?,
            version: u16::from_le_bytes(*version),
            farm_type: FarmType::try_from_primitive(farm_type[0])
                .or(Err(ProgramError::InvalidAccountData))?,
            official: unpack_bool(official)?,
            refdb_index: unpack_option_u32(refdb_index)?,
            refdb_counter: u16::from_le_bytes(*refdb_counter),
            lp_token_ref: unpack_option_key(lp_token_ref)?,
            reward_token_a_ref: unpack_option_key(reward_token_a_ref)?,
            reward_token_b_ref: unpack_option_key(reward_token_b_ref)?,
            router_program_id: Pubkey::new_from_array(*router_program_id),
            farm_program_id: Pubkey::new_from_array(*farm_program_id),
            route: FarmRoute::Raydium {
                farm_id: Pubkey::new_from_array(*farm_id),
                farm_authority: Pubkey::new_from_array(*farm_authority),
                farm_lp_token_account: Pubkey::new_from_array(*farm_lp_token_account),
                farm_reward_token_a_account: Pubkey::new_from_array(*farm_reward_token_a_account),
                farm_reward_token_b_account: unpack_option_key(farm_reward_token_b_account)?,
            },
        })
    }
}

impl std::fmt::Display for FarmType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            FarmType::SingleReward => write!(f, "SingleReward"),
            FarmType::DualReward => write!(f, "DualReward"),
            FarmType::ProtocolTokenStake => write!(f, "ProtocolTokenStake"),
        }
    }
}

impl std::fmt::Display for Farm {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", to_string(&self).unwrap())
    }
}