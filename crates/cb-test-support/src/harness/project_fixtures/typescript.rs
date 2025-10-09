use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;
use std::path::PathBuf;

use super::ProjectFixtures;

impl ProjectFixtures {
    /// Create a large TypeScript project for performance testing
    pub async fn create_large_typescript_project(
        workspace: &TestWorkspace,
        client: &mut TestClient,
        file_count: usize,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let mut created_files = Vec::new();

        // Create directory structure
        let src_dir = workspace.path().join("src");
        std::fs::create_dir_all(&src_dir)?;

        let components_dir = src_dir.join("components");
        std::fs::create_dir_all(&components_dir)?;

        let services_dir = src_dir.join("services");
        std::fs::create_dir_all(&services_dir)?;

        let utils_dir = src_dir.join("utils");
        std::fs::create_dir_all(&utils_dir)?;

        let types_dir = src_dir.join("types");
        std::fs::create_dir_all(&types_dir)?;

        // Calculate files per directory
        let files_per_dir = file_count / 4;

        // Create type files
        for i in 0..files_per_dir {
            let file_path = types_dir.join(format!("types{}.ts", i));
            let content = format!(
                r#"
export interface Entity{i} {{
    id: number;
    name: string;
    created: Date;
    metadata: Record<string, any>;
}}

export interface EntityFilter{i} {{
    namePattern?: string;
    createdAfter?: Date;
    metadataKeys?: string[];
}}

export type EntityStatus{i} = 'active' | 'inactive' | 'pending' | 'archived';

export interface EntityWithStatus{i} extends Entity{i} {{
    status: EntityStatus{i};
    lastModified: Date;
}}

export class EntityValidator{i} {{
    static validate(entity: Entity{i}): boolean {{
        return entity.id > 0 && entity.name.length > 0;
    }}

    static validateStatus(status: EntityStatus{i}): boolean {{
        return ['active', 'inactive', 'pending', 'archived'].includes(status);
    }}
}}
"#,
                i = i
            );

            client
                .call_tool(
                    "create_file",
                    json!({
                        "file_path": file_path.to_string_lossy(),
                        "content": content
                    }),
                )
                .await?;

            created_files.push(file_path);
        }

        // Create utility files
        for i in 0..files_per_dir {
            let file_path = utils_dir.join(format!("utils{}.ts", i));
            let content = format!(
                r#"
import {{ Entity{i}, EntityFilter{i}, EntityStatus{i}, EntityValidator{i} }} from '../types/types{i}';

export class EntityUtils{i} {{
    static formatName(entity: Entity{i}): string {{
        return entity.name.charAt(0).toUpperCase() + entity.name.slice(1);
    }}

    static filterEntities(entities: Entity{i}[], filter: EntityFilter{i}): Entity{i}[] {{
        return entities.filter(entity => {{
            if (filter.namePattern && !entity.name.includes(filter.namePattern)) {{
                return false;
            }}
            if (filter.createdAfter && entity.created < filter.createdAfter) {{
                return false;
            }}
            if (filter.metadataKeys) {{
                const hasAllKeys = filter.metadataKeys.every(key => key in entity.metadata);
                if (!hasAllKeys) return false;
            }}
            return EntityValidator{i}.validate(entity);
        }});
    }}

    static sortByName(entities: Entity{i}[]): Entity{i}[] {{
        return [...entities].sort((a, b) => a.name.localeCompare(b.name));
    }}

    static groupByStatus(entities: Entity{i}[]): Map<string, Entity{i}[]> {{
        const groups = new Map<string, Entity{i}[]>();
        for (const entity of entities) {{
            const status = 'status' in entity ? (entity as any).status : 'unknown';
            if (!groups.has(status)) {{
                groups.set(status, []);
            }}
            groups.get(status)!.push(entity);
        }}
        return groups;
    }}
}}

export function createEntity{i}(name: string, metadata: Record<string, any> = {{}}): Entity{i} {{
    return {{
        id: Math.floor(Math.random() * 1000000),
        name,
        created: new Date(),
        metadata
    }};
}}

export async function batchCreateEntities{i}(names: string[]): Promise<Entity{i}[]> {{
    return names.map(name => createEntity{i}(name));
}}
"#,
                i = i
            );

            client
                .call_tool(
                    "create_file",
                    json!({
                        "file_path": file_path.to_string_lossy(),
                        "content": content
                    }),
                )
                .await?;

            created_files.push(file_path);
        }

        // Create service files
        for i in 0..files_per_dir {
            let file_path = services_dir.join(format!("service{}.ts", i));
            let content = format!(
                r#"
import {{ Entity{i}, EntityFilter{i}, EntityWithStatus{i}, EntityStatus{i} }} from '../types/types{i}';
import {{ EntityUtils{i}, createEntity{i}, batchCreateEntities{i} }} from '../utils/utils{i}';

export class EntityService{i} {{
    private entities: Map<number, Entity{i}> = new Map();
    private cache: Map<string, Entity{i}[]> = new Map();

    async loadEntity{i}(id: number): Promise<Entity{i} | null> {{
        if (this.entities.has(id)) {{
            return this.entities.get(id) || null;
        }}

        // Simulate async loading
        await new Promise(resolve => setTimeout(resolve, Math.random() * 100));

        const entity = createEntity{i}(`Entity_${{id}}`);
        entity.id = id;
        this.entities.set(id, entity);
        return entity;
    }}

    async saveEntity{i}(entity: Entity{i}): Promise<boolean> {{
        try {{
            this.entities.set(entity.id, entity);
            this.invalidateCache();
            return true;
        }} catch (error) {{
            console.error('Failed to save entity:', error);
            return false;
        }}
    }}

    async findEntities{i}(filter: EntityFilter{i}): Promise<Entity{i}[]> {{
        const cacheKey = JSON.stringify(filter);
        if (this.cache.has(cacheKey)) {{
            return this.cache.get(cacheKey) || [];
        }}

        const allEntities = Array.from(this.entities.values());
        const filtered = EntityUtils{i}.filterEntities(allEntities, filter);
        this.cache.set(cacheKey, filtered);
        return filtered;
    }}

    async deleteEntity{i}(id: number): Promise<boolean> {{
        const deleted = this.entities.delete(id);
        if (deleted) {{
            this.invalidateCache();
        }}
        return deleted;
    }}

    private invalidateCache(): void {{
        this.cache.clear();
    }}

    async bulkCreate{i}(count: number): Promise<Entity{i}[]> {{
        const names = Array.from({{ length: count }}, (_, i) => `BulkEntity_${{i}}`);
        const entities = await batchCreateEntities{i}(names);

        for (const entity of entities) {{
            this.entities.set(entity.id, entity);
        }}

        return entities;
    }}

    getStatistics(): {{ total: number; cacheSize: number }} {{
        return {{
            total: this.entities.size,
            cacheSize: this.cache.size
        }};
    }}
}}
"#,
                i = i
            );

            client
                .call_tool(
                    "create_file",
                    json!({
                        "file_path": file_path.to_string_lossy(),
                        "content": content
                    }),
                )
                .await?;

            created_files.push(file_path);
        }

        // Create component files
        for i in 0..files_per_dir {
            let file_path = components_dir.join(format!("component{}.ts", i));
            let content = format!(
                r#"
import {{ Entity{i}, EntityFilter{i}, EntityWithStatus{i} }} from '../types/types{i}';
import {{ EntityService{i} }} from '../services/service{i}';
import {{ EntityUtils{i} }} from '../utils/utils{i}';

export interface ComponentProps{i} {{
    entities: Entity{i}[];
    onEntitySelect?: (entity: Entity{i}) => void;
    onEntityUpdate?: (entity: Entity{i}) => Promise<void>;
    filter?: EntityFilter{i};
}}

export class EntityComponent{i} {{
    private service: EntityService{i};
    private selectedEntity: Entity{i} | null = null;

    constructor(private props: ComponentProps{i}) {{
        this.service = new EntityService{i}();
    }}

    async initialize(): Promise<void> {{
        try {{
            await this.loadInitialData();
            this.setupEventHandlers();
        }} catch (error) {{
            console.error('Component initialization failed:', error);
        }}
    }}

    private async loadInitialData(): Promise<void> {{
        if (this.props.filter) {{
            const filteredEntities = await this.service.findEntities{i}(this.props.filter);
            this.props.entities.push(...filteredEntities);
        }}
    }}

    private setupEventHandlers(): void {{
        // Simulate event handling
        console.log('Event handlers setup for Component{i}');
    }}

    async selectEntity{i}(id: number): Promise<void> {{
        const entity = await this.service.loadEntity{i}(id);
        if (entity) {{
            this.selectedEntity = entity;
            if (this.props.onEntitySelect) {{
                this.props.onEntitySelect(entity);
            }}
        }}
    }}

    async updateEntity{i}(updates: Partial<Entity{i}>): Promise<boolean> {{
        if (!this.selectedEntity) return false;

        const updatedEntity = {{ ...this.selectedEntity, ...updates }};
        const success = await this.service.saveEntity{i}(updatedEntity);

        if (success && this.props.onEntityUpdate) {{
            await this.props.onEntityUpdate(updatedEntity);
            this.selectedEntity = updatedEntity;
        }}

        return success;
    }}

    render(): string {{
        const sortedEntities = EntityUtils{i}.sortByName(this.props.entities);
        return `<div>Component{i} with ${{sortedEntities.length}} entities</div>`;
    }}

    destroy(): void {{
        this.selectedEntity = null;
        console.log('Component{i} destroyed');
    }}
}}

export function createComponent{i}(entities: Entity{i}[], filter?: EntityFilter{i}): EntityComponent{i} {{
    return new EntityComponent{i}({{ entities, filter }});
}}
"#,
                i = i
            );

            client
                .call_tool(
                    "create_file",
                    json!({
                        "file_path": file_path.to_string_lossy(),
                        "content": content
                    }),
                )
                .await?;

            created_files.push(file_path);
        }

        Ok(created_files)
    }
}