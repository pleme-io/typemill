use codebuddy_workspaces::{Workspace, WorkspaceManager};

#[test]
fn test_user_isolation() {
    let manager = WorkspaceManager::new();

    let workspace_a = Workspace {
        id: "ws1".into(),
        language: "rust".into(),
        project_name: "project-a".into(),
        agent_url: "http://ws1.test".into(),
    };
    let workspace_b = Workspace {
        id: "ws2".into(),
        language: "python".into(),
        project_name: "project-b".into(),
        agent_url: "http://ws2.test".into(),
    };

    manager.register("user_a", workspace_a);
    manager.register("user_b", workspace_b);

    // User A can only see their workspace
    let user_a_workspaces = manager.list("user_a");
    assert_eq!(user_a_workspaces.len(), 1);
    assert_eq!(user_a_workspaces[0].id, "ws1");
    assert!(manager.get("user_a", "ws1").is_some());
    assert!(manager.get("user_a", "ws2").is_none());

    // User B can only see their workspace
    let user_b_workspaces = manager.list("user_b");
    assert_eq!(user_b_workspaces.len(), 1);
    assert_eq!(user_b_workspaces[0].id, "ws2");
    assert!(manager.get("user_b", "ws2").is_some());
    assert!(manager.get("user_b", "ws1").is_none());
}

#[test]
fn test_same_workspace_id_different_users() {
    let manager = WorkspaceManager::new();

    let workspace_a = Workspace {
        id: "project".into(),
        language: "rust".into(),
        project_name: "rust-project".into(),
        agent_url: "http://rust.test".into(),
    };
    let workspace_b = Workspace {
        id: "project".into(),
        language: "python".into(),
        project_name: "python-project".into(),
        agent_url: "http://python.test".into(),
    };

    manager.register("user_a", workspace_a);
    manager.register("user_b", workspace_b);

    // Both users can have workspaces with the same ID
    let ws_a = manager.get("user_a", "project").unwrap();
    let ws_b = manager.get("user_b", "project").unwrap();

    assert_eq!(ws_a.language, "rust");
    assert_eq!(ws_b.language, "python");
}
