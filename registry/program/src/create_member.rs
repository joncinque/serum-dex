use serum_common::pack::Pack;
use serum_registry::access_control;
use serum_registry::accounts::{Member, MemberBooks, Watchtower};
use serum_registry::error::{RegistryError, RegistryErrorCode};
use solana_program::info;
use solana_sdk::account_info::{next_account_info, AccountInfo};
use solana_sdk::pubkey::Pubkey;

pub fn handler(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    delegate: Pubkey,
    watchtower: Watchtower,
) -> Result<(), RegistryError> {
    info!("handler: create_member");

    let acc_infos = &mut accounts.iter();

    let beneficiary_acc_info = next_account_info(acc_infos)?;
    let member_acc_info = next_account_info(acc_infos)?;
    let entity_acc_info = next_account_info(acc_infos)?;
    let registrar_acc_info = next_account_info(acc_infos)?;
    let rent_acc_info = next_account_info(acc_infos)?;

    access_control(AccessControlRequest {
        beneficiary_acc_info,
        member_acc_info,
        entity_acc_info,
        registrar_acc_info,
        rent_acc_info,
        program_id,
    })?;

    Member::unpack_unchecked_mut(
        &mut member_acc_info.try_borrow_mut_data()?,
        &mut |member: &mut Member| {
            state_transition(StateTransitionRequest {
                beneficiary_acc_info,
                member,
                delegate,
                entity_acc_info,
                registrar_acc_info,
                watchtower,
            })
            .map_err(Into::into)
        },
    )?;

    Ok(())
}

fn access_control(req: AccessControlRequest) -> Result<(), RegistryError> {
    info!("access-control: create_member");

    let AccessControlRequest {
        beneficiary_acc_info,
        member_acc_info,
        entity_acc_info,
        rent_acc_info,
        registrar_acc_info,
        program_id,
    } = req;

    // Authorization.
    if !beneficiary_acc_info.is_signer {
        return Err(RegistryErrorCode::Unauthorized)?;
    }

    // Account validation.
    let rent = access_control::rent(rent_acc_info)?;
    let _ = access_control::registrar(registrar_acc_info, program_id)?;
    let _ = access_control::entity(entity_acc_info, registrar_acc_info, program_id)?;

    // CreateMember specific.
    {
        if !rent.is_exempt(member_acc_info.lamports(), member_acc_info.try_data_len()?) {
            return Err(RegistryErrorCode::NotRentExempt)?;
        }
        // Use unpack_unchecked since the data will be zero initialized
        // and so won't consume the entire slice (since Member has internal
        // state using Vecs).
        let mut data: &[u8] = &member_acc_info.try_borrow_data()?;
        let member = Member::unpack_unchecked(&mut data)?;
        if member_acc_info.owner != program_id {
            return Err(RegistryErrorCode::InvalidAccountOwner)?;
        }
        if member.initialized {
            return Err(RegistryErrorCode::AlreadyInitialized)?;
        }
    }

    Ok(())
}

fn state_transition(req: StateTransitionRequest) -> Result<(), RegistryError> {
    info!("state-transition: create_member");

    let StateTransitionRequest {
        beneficiary_acc_info,
        member,
        delegate,
        entity_acc_info,
        registrar_acc_info,
        watchtower,
    } = req;

    member.initialized = true;
    member.registrar = *registrar_acc_info.key;
    member.entity = *entity_acc_info.key;
    member.beneficiary = *beneficiary_acc_info.key;
    member.generation = 0;
    member.watchtower = watchtower;
    member.books = MemberBooks::new(*beneficiary_acc_info.key, delegate);
    member.last_active_prices = Default::default();

    Ok(())
}

struct AccessControlRequest<'a, 'b> {
    beneficiary_acc_info: &'a AccountInfo<'b>,
    member_acc_info: &'a AccountInfo<'b>,
    entity_acc_info: &'a AccountInfo<'b>,
    registrar_acc_info: &'a AccountInfo<'b>,
    rent_acc_info: &'a AccountInfo<'b>,
    program_id: &'a Pubkey,
}

struct StateTransitionRequest<'a, 'b, 'c> {
    beneficiary_acc_info: &'a AccountInfo<'b>,
    registrar_acc_info: &'a AccountInfo<'b>,
    entity_acc_info: &'a AccountInfo<'b>,
    member: &'c mut Member,
    delegate: Pubkey,
    watchtower: Watchtower,
}
