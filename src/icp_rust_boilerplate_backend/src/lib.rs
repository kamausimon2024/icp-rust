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
struct User{
   
    id:u64,
    name:String,
    email:String,
    phonenumber:String,
    industry:String,
    skills:String,
    joined_date:u64,
}
impl Storable for User{
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for User{
    const MAX_SIZE: u32 = 512;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMEORY_MANAGER:RefCell<MemoryManager<DefaultMemoryImpl>>=RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );
    static ID_COUNTER:RefCell<IdCell>=RefCell::new(
        IdCell::init(MEMEORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))),0).expect("Cannot create a counter")
    );
    static USERS_STORAGE:RefCell<StableBTreeMap<u64,User,Memory>>=RefCell::new(StableBTreeMap::init(
        MEMEORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));
}

#[derive(candid::CandidType,Clone,Serialize,Deserialize,Default)]

struct UserPayload{
    name:String,
    email:String,
    phonenumber:String,
    industry:String,
    skills:String,

}

#[derive(candid::CandidType,Clone,Serialize,Deserialize,Default)]

struct UserUpdatePayload{
    name:String,
    email:String,
    phonenumber:String,
    industry:String,
    skills:String,

    userid:u64
}
#[derive(candid::CandidType,Serialize,Deserialize,Default)]

struct SearchPayload{
    userid:u64,
}


#[derive(candid::CandidType,Serialize,Deserialize,Default)]
struct DeleteUserPayload{
    userid:u64
}

#[derive(candid::CandidType,Deserialize,Serialize)]
enum Errors{
    USERALREADYEXISTS{msg:String},
    NotFound{msg:String},
    TansporterNameAlradyEXist{msg:String},
    OnyOwner{msg:String},
    MissingCredentials{msg:String}
}

#[ic_cdk::update]
fn create_user_profile(payload: UserPayload) -> Result<User, String> {
    // Validate the payload to ensure that the required fields are present
    if  payload.email.is_empty()
        ||payload.name.is_empty()
        ||payload.industry.is_empty()
        ||payload.phonenumber.is_empty()
        ||payload.skills.is_empty()

    {
        return Err("All fields are required".to_string());
    }

    // Validate the payload to ensure that the email format is correct
    if !payload.email.contains('@') {
        return Err("enter correct email format".to_string());
    }

    // Ensure email address uniqueness and ownername and also transport name
    let email_exists:bool = USERS_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .any(|(_, val)| val.email == payload.email)
    });
    if email_exists {
        return Err("Email already exists".to_string());
    }

   let username_exists:bool=USERS_STORAGE.with(|storage| {
    storage
        .borrow()
        .iter()
        .any(|(_,val)| val.name == payload.name)
});
if username_exists {
    return Err("The username already exists".to_string());
}
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Cannot increment ID counter");

    let new_user = User {
        id,
        name:payload.name,
        email:payload.email,
        phonenumber: payload.phonenumber,
        skills:payload.skills,
        industry:payload.industry,
       
        joined_date: time(),
       
    };

    USERS_STORAGE.with(|storage| storage.borrow_mut().insert(id, new_user.clone()));

    Ok(new_user)
}

#[ic_cdk::update]

fn user_update_details(payload:UserUpdatePayload)->Result<User,String>{
    if  payload.email.is_empty()
    ||payload.name.is_empty()
    ||payload.industry.is_empty()
    ||payload.phonenumber.is_empty()
    ||payload.skills.is_empty()

  {
    return Err("All fields are required".to_string());
  }
     // Validate the payload to ensure that the email format is correct
     if !payload.email.contains('@') {
        return Err("Invalid email format".to_string());
    }
    
match USERS_STORAGE.with(|service|service.borrow().get(&payload.userid)){
    Some(mut us)=>{
                        us.name=payload.name;
                        us.email=payload.email;
                        us.industry=payload.industry;
                        us.phonenumber=payload.phonenumber;
                        us.skills=payload.skills;
                       
                        do_insert(&us);
                        Ok(us)
                        
    }
    None=>Err("could not update user details".to_string()),
}}
//get all users
#[ic_cdk::query]
fn get_all_users() -> Result<Vec<User>, String> {

    let users = USERS_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .map(|(_, trans)| trans.clone())
            .collect::<Vec<User>>()
    });

    if  users.is_empty() {
        return Err("No users  found.".to_string());
    }

    else {
        Ok(users)
    }
  
}

//user update his details


#[ic_cdk::query]
fn get_user(payload:SearchPayload)->Result<User,String>{
    let user = USERS_STORAGE.with(|storage| storage.borrow().get(&payload.userid));
    match user {
        Some(user) => Ok(user),
        None => Err("user with the provided ID does not exist.".to_string()),
    }
   
}
#[ic_cdk::update]
  fn delete_user_profile(payload:DeleteUserPayload)->Result<String,String>{
 //verify its the owner
   
   let user =USERS_STORAGE.with(|storage| storage.borrow().get(&payload.userid));
    match user {
        Some(_) => (),
        None => return Err("failed.".to_string()),
    }
    match USERS_STORAGE.with(|storage|storage.borrow_mut().remove(&payload.userid)){
        Some(_val)=>Ok("tou have opted out, thank you".to_string()),
        None=>Err("failed to opt out".to_string(),)
    }
  }

  

fn do_insert(us:&User){
    USERS_STORAGE.with(|service|service.borrow_mut().insert(us.id,us.clone()));
}
ic_cdk::export_candid!();
