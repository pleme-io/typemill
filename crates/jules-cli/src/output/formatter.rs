use crate::output::table::print_table;
use anyhow::Result;
use jules_api::types::{
    ActivitiesResponse, Activity, Session, SessionsResponse, Source, SourcesResponse,
};
use serde::Serialize;

fn print_json<T: Serialize + ?Sized>(value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value)?;
    println!("{}", json);
    Ok(())
}

// --- Source Formatters ---

pub fn print_sources_response(response: &SourcesResponse, format: &str) -> Result<()> {
    match format {
        "json" => print_json(response)?,
        _ => {
            let headers = vec!["ID", "Name", "Language", "Description"];
            let rows: Vec<Vec<String>> = response
                .sources
                .iter()
                .map(|s| {
                    vec![
                        s.id.clone(),
                        s.name.clone(),
                        s.language.clone(),
                        s.description.clone(),
                    ]
                })
                .collect();
            print_table(headers, rows);
            if let Some(token) = &response.next_page_token {
                println!("\nNext page token: {}", token);
            }
        }
    }
    Ok(())
}

pub fn print_source(source: &Source, format: &str) -> Result<()> {
    match format {
        "json" => print_json(source)?,
        _ => {
            let headers = vec!["ID", "Name", "Language", "Description"];
            let rows = vec![vec![
                source.id.clone(),
                source.name.clone(),
                source.language.clone(),
                source.description.clone(),
            ]];
            print_table(headers, rows);
        }
    }
    Ok(())
}

// --- Session Formatters ---

pub fn print_sessions_response(response: &SessionsResponse, format: &str) -> Result<()> {
    match format {
        "json" => print_json(response)?,
        _ => {
            let headers = vec!["ID", "Source ID", "State", "Created At"];
            let rows: Vec<Vec<String>> = response
                .sessions
                .iter()
                .map(|s| {
                    vec![
                        s.id.clone(),
                        s.source_id.clone(),
                        s.state.clone(),
                        s.created_at.clone(),
                    ]
                })
                .collect();
            print_table(headers, rows);
            if let Some(token) = &response.next_page_token {
                println!("\nNext page token: {}", token);
            }
        }
    }
    Ok(())
}

pub fn print_session(session: &Session, format: &str) -> Result<()> {
    match format {
        "json" => print_json(session)?,
        _ => {
            let headers = vec!["ID", "Source ID", "State", "Created At"];
            let rows = vec![vec![
                session.id.clone(),
                session.source_id.clone(),
                session.state.clone(),
                session.created_at.clone(),
            ]];
            print_table(headers, rows);
        }
    }
    Ok(())
}

// --- Activity Formatters ---

pub fn print_activities_response(response: &ActivitiesResponse, format: &str) -> Result<()> {
    match format {
        "json" => print_json(response)?,
        _ => {
            let headers = vec!["ID", "Type", "Created At", "Content"];
            let rows: Vec<Vec<String>> = response
                .activities
                .iter()
                .map(|a| {
                    vec![
                        a.id.clone(),
                        a.r#type.clone(),
                        a.created_at.clone(),
                        a.content.trim().to_string(),
                    ]
                })
                .collect();
            print_table(headers, rows);
            if let Some(token) = &response.next_page_token {
                println!("\nNext page token: {}", token);
            }
        }
    }
    Ok(())
}