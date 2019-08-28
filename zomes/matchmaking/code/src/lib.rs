#![feature(proc_macro_hygiene)]
#[macro_use]
extern crate hdk;
extern crate hdk_proc_macros;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate holochain_json_derive;

use hdk::{
    entry_definition::ValidatingEntryType,
    error::ZomeApiResult,
};
use hdk::holochain_core_types::{
    entry::Entry,
    dna::entry_types::Sharing,
};

use hdk::holochain_json_api::{
    json::JsonString,
    error::JsonError
};

use hdk::holochain_persistence_api::{
    cas::content::Address
};

use hdk_proc_macros::zome;

// see https://developer.holochain.org/api/0.0.18-alpha1/hdk/ for info on using the hdk library

// This is a sample zome that defines an entry type "MyEntry" that can be committed to the
// agent's chain via the exposed function create_my_entry

#[derive(Serialize, Deserialize, Debug, DefaultJson, Clone)]
pub struct GameProposal {
    pub agent: Address,
    pub message: String,
    pub timestamp: u32,
}

#[zome]
mod my_zome {

    #[genesis]
    fn genesis() {
        Ok(())
    }

    #[entry_def]
     fn game_proposal_def() -> ValidatingEntryType {
        entry!(
            name: "game_proposal",
            description: "Represents an agent advertizing they wish to play a game at this time",
            sharing: Sharing::Public,
            validation_package: || {
                hdk::ValidationPackageDefinition::Entry
            },
            validation: | _validation_data: hdk::EntryValidationData<MyEntry>| {
                match validation_data {
                    // only match if the entry is being created (not modified or deleted)
                    EntryValidationData::Create{ entry, validation_data } => {
                        let game_proposal = GameProposal::from(entry);
                        if validation_data.sources().contains(&game_proposal.agent) {
                            Ok(())
                        } else {
                            Err("Cannot author a proposal from another agent".into())
                        }
                    },
                    EntryValidationData::Delete{ old_entry, old_entry_header, validation_data } => {
                        // should update to only the author can delete
                        let game_proposal = GameProposal::from(old_entry);
                        if validation_data.sources().contains(&game_proposal.agent) {
                            Ok(())
                        } else {
                            Err("Cannot delete a proposal from another agent".into())
                        }
                    },
                    _ => {
                        Err("Cannot modify, only create and delete".into())
                    }
                }
            }
        )
    }

    #[entry_def]
    pub anchor_def() -> ValidatingEntryType {
        entry!(
            name: "anchor",
            description: "Central known location to link from",
            sharing: Sharing::Public, 
            validation_package: || { hdk::ValidationPackageDefinition::Entry },
            validation: | _validation_data: hdk::EntryValidationData<String>| {
                // TODO: check if the anchor is "game_proposals"
                Ok(())
            },
            links: [
                to!(
                    "game_proposal", // this must match exactly the target entry type
                    link_type: "has_proposal", // must use this when creating the link
                    validation_package: || {
                        hdk::ValidationPackageDefinition::Entry
                    },
                    validation: | _validation_data: hdk::LinkValidationData| {
                        // TODO: check if the anchor is "game_proposals"
                        Ok(())
                    }
                )
            ]
        )
    }

    #[zome_fn("hc_public")]
    pub fn handle_create_proposal(message: String, timestamp: u32) -> ZomeApiResult<Address> {
        // create the data as a struct
        let game_proposal_data = GameProposal { 
            agent: AGENT_ADDRESS.to_string().into(),
            message,
            timestamp
        };
        
        // create an entry
        let entry = Entry::App(
            "game_proposal".into(),
            game_proposal_data.into(),
        );
        
        // commit the entry. '?' means return immedietly on error
        let proposal_address = hdk::commit_entry(&entry)?;
        
        // create an anchor entry and commit.
        // The native type is string so we can skip the first step
        let anchor_entry = Entry::App(
            "anchor".into(),
            "game_proposals".into(),
        );
        let anchor_address = hdk::commit_entry(&anchor_entry)?;
        
        // finally link them together
        hdk::link_entries(
            &anchor_address,
            &proposal_address,
            "has_proposal", // the link type, defined on the base entry
            "" // the tag which is not used in this example
        )?;
        
        // return the proposal address
        Ok(proposal_address)
    }

}
