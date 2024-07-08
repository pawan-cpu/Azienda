#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{self, MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::collections::HashMap;
use std::fmt::format;
use std::{borrow::Cow, cell::RefCell};
use ic_stable_structures::storable::Bound;
//use std::collections::*;
use ic_cdk::storage;

/*Defining Memory state and IdCell*/

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;
//type JobStorage = HashMap<u64, Job>;

//Defining the job application struct
#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Job {
       id: u64,             //the id for the job
       title: String,       // the job title
       description: String, //job description
      // employer: String,    // the employer that creates the job
      created_at: u64,  
       applicant_name: Vec<String>,  
       accepted_applicants: Option<String>,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct CreateJob {
   // id: u64,
    title: String,
    description: String,
  //  applicant_name: Vec<String>,
}

//the enumeration for the error
// #[derive(candid::CandidType, Deserialize, Serialize)]
// enum Error {
//     JobNotFound {msg: String},
// }

#[derive(candid::CandidType, Deserialize, Serialize)]
enum JobStatus{
    AcceptJob,
    JobWithdrawn,
    JobCancelled,
}

//next , we implement a trait that must be implemented for a struct that is stored in a stable struct
impl Storable for Job {
    
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: 1024,
        is_fixed_size: false,
    };
}

thread_local! {
    /*this thread local variable holds our cannister's virtual memeory, which enables us to access the memory manager from any part of our code*/
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );
     
     /*this holds our cannister ID counter, allowing us to access it from anywhere in our code*/
    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0).expect("cannot create counter")
    );

    /*this variable holds our cannister storage, enabling access from anywhere in our code*/
    static STORAGE: RefCell<StableBTreeMap<u64, Job, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
        ));
}


/*below is our function to create the job */
#[ic_cdk::update]
fn create_job(job: CreateJob) -> Job {
    let id = ID_COUNTER.with(|counter|{
        let current_value = *counter.borrow().get();
        counter.borrow_mut().set(current_value + 1).expect("Cannot increment id counter");
        current_value + 1
    });

    let job = Job{
        id,
        title: job.title,
        description: job.description,
        created_at: time(),
        applicant_name: vec![],
        accepted_applicants: None,
    };

    STORAGE.with(|storage| storage.borrow_mut().insert(job.id, job.clone()));
    job
}
// fn do_insert(job: &Job) {
//     STORAGE.with(|service| service.borrow_mut().insert(job.id, job.clone()));
// }


/*this is our function to apply for the job*/
#[ic_cdk::update]
fn apply_to_job(job_id: u64, applicant_name: String) -> Result<(), String> {
    STORAGE.with(|storage| {
        let mut job_opt = {
            let mut storage_ref = storage.borrow_mut();
            storage_ref.get(&job_id).clone()
        };

        if let Some(mut job) = job_opt {
            job.applicant_name.push(applicant_name);

            STORAGE.with(|storage| {
                storage.borrow_mut().insert(job.id, job);
            });

            Ok(())
        } else {
            Err(String::from("Job not found"))
        }
    })
}

// fn apply_to_job(job_id: u64, applicant_name: String) -> Result<(), String> {
//   STORAGE.with(|storage| {
//     if let Some(mut job) = storage.borrow_mut().get(&job_id){ //.clone
//         job.applicant_name.push(applicant_name);
//         storage.borrow_mut().insert(job_id, job);
//         Ok(())
//     } else {
//         Err("job Not Found".to_string())
//     }
// })
// }

/* this is our application withdrawn function*/
#[ic_cdk::update]
fn withdraw_application(job_id: u64, applicant_name: String) -> Result<(), String> {
    let mut job_opt = STORAGE.with(|storage| {
        storage.borrow().get(&job_id).clone()
    });

    if let Some(mut job) = job_opt {
        job.applicant_name.retain(|name| name != &applicant_name);

        STORAGE.with(|storage| {
            storage.borrow_mut().insert(job.id, job);
        });

        Ok(())
    } else {
        Err(String::from("Job not found"))
    }
}


// fn withdraw_application(job_id: u64, applicant_name: String) -> Result<(), String> {
//     STORAGE.with(|storage|{
//         if let Some(mut job) = storage.borrow_mut().get(&job_id).clone(){
//             job.applicant_name.retain(|name| name != &applicant_name);
//             storage.borrow_mut().insert(job.id, job);
//             Ok(())
//         }else {
//             Err("job not found".to_string())
//         }
//     })
// }


/* cancel job function*/
#[ic_cdk::update]
fn cancel_job(job_id: u64) -> Result<(), String> {
    STORAGE.with(|storage| {
        if storage.borrow_mut().remove(&job_id).is_some() {
            Ok(())
        }else{
            Err(String::from("Job not found"))
        }
    })
}
 
 /*job acceptance function */
 #[ic_cdk::update]
 fn accept_job(job_id: u64, applicant_name: String) -> Result<(), String> {
    let mut job_opt = STORAGE.with(|storage| {
        storage.borrow().get(&job_id).clone()
    });

    if let Some(mut job) = job_opt {
        if job.applicant_name.contains(&applicant_name) {
            job.accepted_applicants = Some(applicant_name);

            STORAGE.with(|storage| {
                storage.borrow_mut().insert(job.id, job);
            });

            Ok(())
        } else {
            Err(String::from("Applicant not found"))
        }
    } else {
        Err(String::from("Job not found"))
    }
}


 //  fn accept_job(job_id: u64, applicant_name: String) -> Result<(), String>{
//     STORAGE.with(|storage|{
//         if let Some(mut job) = storage.borrow_mut().get(&job_id).clone(){
//             if job.applicant_name.contains(&applicant_name) {
//                 job.accepted_applicants =Some(applicant_name);
//                 storage.borrow_mut().insert(job_id, job);
//                 Ok(())
//             }else{
//                 Err("Application not found".to_string())
//             }
//             }else{
//                 Err("Job not found".to_string())
//             }
//         })
    
//  }

 #[ic_cdk::query]
 fn fetch_job(job_id: u64) -> Result<Job, String> {
    STORAGE.with(|storage|{
        if let Some(job) = storage.borrow().get(&job_id){
            Ok(job.clone())
        } else {
            Err("Job not found".to_string())
        }
    })
 }


 ic_cdk::export_candid!();