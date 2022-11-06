#![allow(unused)]
use crate::modules::git::Git;
use crate::modules::gql_client::CustomizedGqlClient;
use anyhow::Result;
use std::time::Duration;
use tokio::time;
use tokio_cron_scheduler::{Job, JobScheduler};

pub(crate) async fn run_cron(mut sched: JobScheduler) -> Result<()> {
    #[cfg(feature = "signal")]
    sched.shutdown_on_ctrl_c();

    sched.set_shutdown_handler(Box::new(|| {
        Box::pin(async move {
            println!("Shut down done");
        })
    }));

    let mut job = Job::new_async("0 0 14 ? * * *", |uuid, mut l| {
        // let mut job = Job::new_async("0 0/2 * ? * * *", |uuid, mut l| {
        Box::pin(async move {
            println!("I run async, id {:?}", uuid);

            // Initialize GQL Client
            let client = CustomizedGqlClient::new_client();
            // Trigger action to get latest commits of repos
            let result = Git::get_latest_commits(
                &client,
                &dotenv::var("GITHUB_USERNAME")
                    .expect("Username not found"),
                Some(2),
                None,
            )
            .await
            .unwrap();
            println!("Response: {:?}", "OK");

            let next_tick = l.next_tick_for_job(uuid).await;
            match next_tick {
                Ok(Some(ts)) => {
                    println!("Next time is {:?}", ts)
                }
                _ => println!("Could not get next tick for 59s job"),
            }
        })
    })
    .unwrap();

    let job_clone = job.clone();
    let js = sched.clone();
    println!("Job id {:?}", job.guid());
    job.on_start_notification_add(&sched, Box::new(move |job_id, notification_id, type_of_notification| {
        let job_clone = job_clone.clone();
        let js = js.clone();
        Box::pin(async move {
            println!("Job {:?} ran on start notification {:?} ({:?})", job_id, notification_id, type_of_notification);
        })
    })).await?;

    job
        .on_done_notification_add(
            &sched,
            Box::new(|job_id, notification_id, type_of_notification| {
                Box::pin(async move {
                    println!(
                        "Job {:?} completed and ran notification {:?} ({:?})",
                        job_id, notification_id, type_of_notification
                    );
                })
            }),
        )
        .await?;

    let four_s_job_guid = job.guid();
    sched.add(job).await?;

    let start = sched.start().await;
    if start.is_err() {
        println!("Error starting scheduler");
        return Ok(());
    }

    loop {
        sched.tick();
        time::sleep(Duration::from_millis(1000));
    }

    Ok(())
}