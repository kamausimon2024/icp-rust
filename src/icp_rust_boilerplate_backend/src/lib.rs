#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct User {
    id: u64,
    name: String,
    email: String,
    phonenumber: String,
    industry: String,
    skills: String,
    lookingforjob: bool,
    joined_date: u64,
}

impl Storable for User {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for User {
    const MAX_SIZE: u32 = 512;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );
    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0).expect("Cannot create a counter")
    );
    static USERS_STORAGE: RefCell<StableBTreeMap<u64, User, Memory>> = RefCell::new(StableBTreeMap::init(
        MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct UserPayload {
    name: String,
    email: String,
    phonenumber: String,
    industry: String,
    skills: String,
    lookingforjob: bool,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct UserUpdatePayload {
    userid: u64,
    name: String,
    email: String,
    phonenumber: String,
    industry: String,
    skills: String,
    lookingforjob: bool,
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct SearchPayload {
    userid: u64,
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct DeleteUserPayload {
    userid: u64,
}

#[derive(candid::CandidType, Deserialize, Serialize)]
enum Errors {
    UserAlreadyExists { msg: String },
    NotFound { msg: String },
    MissingCredentials { msg: String },
    InvalidEmailFormat { msg: String },
    EmptyFields { msg: String },
}

#[ic_cdk::update]
fn create_user_profile(payload: UserPayload) -> Result<User, Errors> {
    validate_user_payload(&payload)?;

    let email_exists = USERS_STORAGE.with(|storage| {
        storage.borrow().iter().any(|(_, user)| user.email == payload.email)
    });
    if email_exists {
        return Err(Errors::UserAlreadyExists {
            msg: "Email already exists".to_string(),
        });
    }

    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Cannot increment ID counter");

    let new_user = User {
        id,
        name: payload.name,
        email: payload.email,
        phonenumber: payload.phonenumber,
        skills: payload.skills,
        industry: payload.industry,
        lookingforjob: payload.lookingforjob,
        joined_date: time(),
    };

    USERS_STORAGE.with(|storage| storage.borrow_mut().insert(id, new_user.clone()));
    Ok(new_user)
}

#[ic_cdk::query]
fn get_all_users() -> Result<Vec<User>, Errors> {
    let users = USERS_STORAGE.with(|storage| {
        storage.borrow().iter().map(|(_, user)| user.clone()).collect::<Vec<User>>()
    });

    if users.is_empty() {
        Err(Errors::NotFound {
            msg: "No users found".to_string(),
        })
    } else {
        Ok(users)
    }
}

#[ic_cdk::query]
fn get_user(payload: SearchPayload) -> Result<User, Errors> {
    USERS_STORAGE.with(|storage| storage.borrow().get(&payload.userid)).ok_or(Errors::NotFound {
        msg: "User with the provided ID does not exist".to_string(),
    })
}

#[ic_cdk::update]
fn user_update_details(payload: UserUpdatePayload) -> Result<User, Errors> {
    validate_user_update_payload(&payload)?;

    USERS_STORAGE.with(|storage| {
        let mut storage = storage.borrow_mut();
        if let Some(mut user) = storage.get(&payload.userid) {
            user.name = payload.name;
            user.email = payload.email;
            user.phonenumber = payload.phonenumber;
            user.industry = payload.industry;
            user.skills = payload.skills;
            user.lookingforjob = payload.lookingforjob;

            // Remove the old user and re-insert the updated user
            storage.remove(&payload.userid);
            storage.insert(payload.userid, user.clone());
            Ok(user)
        } else {
            Err(Errors::NotFound {
                msg: "User not found".to_string(),
            })
        }
    })
}


#[ic_cdk::update]
fn delete_user_profile(payload: DeleteUserPayload) -> Result<String, Errors> {
    let user_exists = USERS_STORAGE.with(|storage| storage.borrow().contains_key(&payload.userid));

    if !user_exists {
        return Err(Errors::NotFound {
            msg: "User not found".to_string(),
        });
    }

    USERS_STORAGE.with(|storage| storage.borrow_mut().remove(&payload.userid));
    Ok("User deleted successfully".to_string())
}

fn validate_user_payload(payload: &UserPayload) -> Result<(), Errors> {
    if payload.name.is_empty()
        || payload.email.is_empty()
        || payload.phonenumber.is_empty()
        || payload.industry.is_empty()
        || payload.skills.is_empty()
    {
        return Err(Errors::EmptyFields {
            msg: "All fields are required".to_string(),
        });
    }

    if !payload.email.contains('@') {
        return Err(Errors::InvalidEmailFormat {
            msg: "Invalid email format".to_string(),
        });
    }

    Ok(())
}

fn validate_user_update_payload(payload: &UserUpdatePayload) -> Result<(), Errors> {
    if payload.name.is_empty()
        || payload.email.is_empty()
        || payload.phonenumber.is_empty()
        || payload.industry.is_empty()
        || payload.skills.is_empty()
    {
        return Err(Errors::EmptyFields {
            msg: "All fields are required".to_string(),
        });
    }

    if !payload.email.contains('@') {
        return Err(Errors::InvalidEmailFormat {
            msg: "Invalid email format".to_string(),
        });
    }

    Ok(())
}

ic_cdk::export_candid!();