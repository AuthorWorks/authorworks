# Technical Specification: 2F - Editor Service

## Overview

The Editor Service provides the backend functionality for the AuthorWorks collaborative text editor. It enables real-time collaborative editing, advanced formatting, content management, and integration with the AI-assisted writing features. This service is built on top of rustpad for operational transformation and real-time collaboration, with Slate.js for the rich text editing frontend interface.

## Objectives

- Enable real-time collaborative editing of documents
- Provide rich text formatting and structured document editing
- Support versioning and document history
- Implement operational transforms for conflict resolution
- Integrate with AI-assisted writing and generation features
- Support offline editing and synchronization
- Provide document export in multiple formats

## Requirements

### 1. Core Editor Data Models

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditSession {
    pub id: Uuid,
    pub document_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub active_users: Vec<ActiveUser>,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveUser {
    pub user_id: Uuid,
    pub joined_at: DateTime<Utc>,
    pub cursor_position: Option<CursorPosition>,
    pub last_activity: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub path: Vec<usize>,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSnapshot {
    pub id: Uuid,
    pub document_id: Uuid,
    pub version: i64,
    pub content: Value,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIAssistRequest {
    pub id: Uuid,
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub document_id: Uuid,
    pub prompt: String,
    pub context: String,
    pub selection: Option<TextSelection>,
    pub created_at: DateTime<Utc>,
    pub status: AIRequestStatus,
    pub result: Option<String>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSelection {
    pub start_path: Vec<usize>,
    pub start_offset: usize,
    pub end_path: Vec<usize>,
    pub end_offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AIRequestStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSettings {
    pub user_id: Uuid,
    pub theme: String,
    pub font_size: i32,
    pub line_spacing: f32,
    pub spell_check_enabled: bool,
    pub grammar_check_enabled: bool,
    pub autosave_interval_seconds: i32,
    pub custom_settings: HashMap<String, String>,
}
```

### 2. Database Schema

```sql
CREATE TABLE edit_sessions (
    id UUID PRIMARY KEY,
    document_id UUID NOT NULL REFERENCES content.chapters(id),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    status VARCHAR(20) NOT NULL
);

CREATE TABLE active_users (
    session_id UUID NOT NULL REFERENCES edit_sessions(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),
    joined_at TIMESTAMP WITH TIME ZONE NOT NULL,
    cursor_path JSONB,
    cursor_offset INTEGER,
    last_activity TIMESTAMP WITH TIME ZONE NOT NULL,
    PRIMARY KEY (session_id, user_id)
);

CREATE TABLE document_snapshots (
    id UUID PRIMARY KEY,
    document_id UUID NOT NULL,
    version BIGINT NOT NULL,
    content JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_by UUID NOT NULL REFERENCES users(id),
    comment TEXT,
    UNIQUE (document_id, version)
);

CREATE TABLE ai_assist_requests (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES edit_sessions(id),
    user_id UUID NOT NULL REFERENCES users(id),
    document_id UUID NOT NULL,
    prompt TEXT NOT NULL,
    context TEXT NOT NULL,
    selection JSONB,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    status VARCHAR(20) NOT NULL,
    result TEXT,
    completed_at TIMESTAMP WITH TIME ZONE
);

CREATE TABLE editor_settings (
    user_id UUID PRIMARY KEY REFERENCES users(id),
    theme VARCHAR(50) NOT NULL,
    font_size INTEGER NOT NULL,
    line_spacing REAL NOT NULL,
    spell_check_enabled BOOLEAN NOT NULL,
    grammar_check_enabled BOOLEAN NOT NULL,
    autosave_interval_seconds INTEGER NOT NULL,
    custom_settings JSONB NOT NULL
);

-- Indexes
CREATE INDEX idx_snapshots_document ON document_snapshots(document_id);
CREATE INDEX idx_snapshots_version ON document_snapshots(document_id, version);
CREATE INDEX idx_ai_requests_session ON ai_assist_requests(session_id);
CREATE INDEX idx_ai_requests_status ON ai_assist_requests(status);
```

### 3. API Endpoints

```
# Session Management
POST   /v1/editor/sessions                      - Create editing session
GET    /v1/editor/sessions/{id}                 - Get session details
PUT    /v1/editor/sessions/{id}/close           - Close session
GET    /v1/editor/sessions/{id}/users           - Get active users in session
POST   /v1/editor/sessions/{id}/join            - Join session
POST   /v1/editor/sessions/{id}/leave           - Leave session
PUT    /v1/editor/sessions/{id}/cursor          - Update cursor position

# Document Management
GET    /v1/editor/documents/{id}                - Get document content
GET    /v1/editor/documents/{id}/versions       - List document versions
GET    /v1/editor/documents/{id}/snapshots      - List document snapshots
POST   /v1/editor/documents/{id}/snapshots      - Create document snapshot
GET    /v1/editor/snapshots/{id}                - Get snapshot
POST   /v1/editor/documents/{id}/export         - Export document

# AI Assistance
POST   /v1/editor/ai/assist                     - Request AI assistance
GET    /v1/editor/ai/requests/{id}              - Get AI request status/result
GET    /v1/editor/ai/history                    - Get AI request history

# Settings
GET    /v1/editor/settings                      - Get user's editor settings
PUT    /v1/editor/settings                      - Update user's editor settings

# WebSocket endpoint for real-time collaborative editing
WS     /v1/editor/sessions/{id}/ws              - WebSocket connection
```

### 4. Document Structure

The editor will use a structured document format based on Slate.js schema:

```typescript
// TypeScript interface shown for clarity
interface Node {
  id: string;
  type: string;
  children: Node[];
  attributes?: Record<string, any>;
}

interface ElementNode extends Node {
  type: 'paragraph' | 'heading' | 'blockquote' | 'list' | 'list-item' | 'image' | 'table' | 'table-row' | 'table-cell';
}

interface TextNode extends Node {
  type: 'text';
  text: string;
  marks?: Mark[];
}

interface Mark {
  type: 'bold' | 'italic' | 'underline' | 'code' | 'comment' | 'highlight';
  data?: Record<string, any>;
}

interface Document {
  version: number;
  children: Node[];
  metadata: {
    title: string;
    lastModified: string;
    lastModifiedBy: string;
    createdAt: string;
    createdBy: string;
  };
}
```

### 5. Rustpad Integration

The Editor Service will leverage rustpad as the foundation for the collaborative editing backend. Rustpad provides efficient operational transformation (OT) for plain text. We'll extend it to support Slate.js's rich text structure.

```rust
// Extend rustpad's Server to support our needs
pub struct EditorServer {
    rustpad_server: rustpad::Server<SlateDocument>,
    document_repository: Arc<dyn DocumentRepository>,
    snapshot_repository: Arc<dyn SnapshotRepository>,
}

impl EditorServer {
    pub fn new(
        document_repository: Arc<dyn DocumentRepository>,
        snapshot_repository: Arc<dyn SnapshotRepository>,
    ) -> Self {
        // Initialize with custom document type
        let rustpad_server = rustpad::Server::new();
        
        Self {
            rustpad_server,
            document_repository,
            snapshot_repository,
        }
    }
    
    pub async fn create_session(&mut self, document_id: Uuid) -> Result<String, EditorError> {
        // Fetch initial document content from repository
        let document = self.document_repository.find_by_id(&document_id).await?;
        
        // Convert document to SlateDocument format
        let slate_document = SlateDocument::from_json(&document.content)?;
        
        // Create session in rustpad server
        let session_id = self.rustpad_server.create_session(document_id.to_string(), slate_document);
        
        Ok(session_id)
    }
    
    pub async fn join_session(
        &self,
        session_id: &str,
        user_id: Uuid,
    ) -> impl Stream<Item = ServerMessage> + Sink<ClientMessage, Error = EditorError> {
        // Create bidirectional channel for communication with rustpad
        let (client_tx, client_rx) = mpsc::channel(100);
        let (server_tx, server_rx) = mpsc::channel(100);
        
        // Join rustpad session
        let rustpad_stream = self.rustpad_server.join_session(
            session_id, 
            user_id.to_string(),
        );
        
        // Handle incoming messages from rustpad
        let stream_handle = tokio::spawn(async move {
            // Convert rustpad messages to our message format
            // and send them to the client
        });
        
        // Handle outgoing messages to rustpad
        let sink_handle = tokio::spawn(async move {
            // Convert our message format to rustpad messages
            // and send them to rustpad
        });
        
        // Return bidirectional stream
        Box::pin(stream_sink)
    }
    
    pub async fn snapshot(&self, session_id: &str, user_id: Uuid) -> Result<(), EditorError> {
        // Get current document state from rustpad
        let document = self.rustpad_server.get_document(session_id)?;
        
        // Convert to JSON representation
        let content = document.to_json()?;
        
        // Create a snapshot
        let snapshot = DocumentSnapshot {
            id: Uuid::new_v4(),
            document_id: Uuid::parse_str(&session_id)?,
            version: document.get_version(),
            content,
            created_at: Utc::now(),
            created_by: user_id,
            comment: Some("Manual snapshot".to_string()),
        };
        
        // Save to repository
        self.snapshot_repository.create(snapshot).await?;
        
        Ok(())
    }
    
    // Additional methods for managing sessions, users, etc.
}

// Define custom document type for rustpad
pub struct SlateDocument {
    content: Value,
    version: i64,
}

impl rustpad::Document for SlateDocument {
    type Operation = SlateOperation;
    
    fn apply(&mut self, operation: Self::Operation) -> Result<(), rustpad::OperationError> {
        // Apply the slate operation to the document
        // This transforms the operation into a sequence of changes to the document
        match operation {
            SlateOperation::Insert { path, offset, text } => {
                // Find the node at the given path
                let node = self.find_node_mut(&path)?;
                
                // Apply the insert operation
                if let Some(text_node) = node.as_text_mut() {
                    // Insert text at the specified offset
                    text_node.insert_text(offset, &text)?;
                } else {
                    return Err(rustpad::OperationError::InvalidOperation);
                }
            },
            SlateOperation::Delete { path, offset, count } => {
                // Find the node at the given path
                let node = self.find_node_mut(&path)?;
                
                // Apply the delete operation
                if let Some(text_node) = node.as_text_mut() {
                    // Delete text at the specified offset
                    text_node.delete_text(offset, count)?;
                } else {
                    return Err(rustpad::OperationError::InvalidOperation);
                }
            },
            // Handle other operation types
        }
        
        // Increment document version
        self.version += 1;
        
        Ok(())
    }
    
    fn compose(
        a: Self::Operation,
        b: Self::Operation,
    ) -> Result<Self::Operation, rustpad::OperationError> {
        // Compose two operations into a single operation
        // This is used for optimizing operation history
        todo!("Implement compose for SlateOperation")
    }
    
    fn transform(
        a: Self::Operation,
        b: Self::Operation,
    ) -> Result<(Self::Operation, Self::Operation), rustpad::OperationError> {
        // Transform operations a and b into a' and b'
        // This is the core of operational transformation
        todo!("Implement transform for SlateOperation")
    }
}

// Define custom operation type for slate documents
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SlateOperation {
    Insert { path: Vec<usize>, offset: usize, text: String },
    Delete { path: Vec<usize>, offset: usize, count: usize },
    Split { path: Vec<usize>, offset: usize },
    Merge { path: Vec<usize> },
    Move { source_path: Vec<usize>, destination_path: Vec<usize> },
    AddMark { path: Vec<usize>, offset: usize, length: usize, mark: Mark },
    RemoveMark { path: Vec<usize>, offset: usize, length: usize, mark_type: String },
    SetNode { path: Vec<usize>, properties: HashMap<String, Value> },
    InsertNode { path: Vec<usize>, node: Value },
    RemoveNode { path: Vec<usize> },
}
```

### 6. WebSocket Communication

The real-time collaboration will be implemented using WebSockets, building on rustpad's collaboration model:

```rust
pub struct WebSocketHandler {
    editor_server: Arc<RwLock<EditorServer>>,
    active_user_repository: Arc<dyn ActiveUserRepository>,
}

impl WebSocketHandler {
    pub async fn handle_connection(
        &self,
        socket: WebSocket,
        session_id: String,
        user_id: Uuid,
    ) -> Result<(), EditorError> {
        // Register user as connected to session
        self.active_user_repository.add_user_to_session(&Uuid::parse_str(&session_id)?, &user_id).await?;
        
        // Split socket into sender and receiver
        let (mut sender, mut receiver) = socket.split();
        
        // Join editor session
        let mut editor_stream = {
            let editor_server = self.editor_server.write().await;
            editor_server.join_session(&session_id, user_id).await
        };
        
        // Forward messages from client to editor server
        let client_to_server = async {
            while let Some(Ok(message)) = receiver.next().await {
                if let Ok(text) = message.to_str() {
                    let client_message: ClientMessage = serde_json::from_str(text)?;
                    editor_stream.send(client_message).await?;
                }
            }
            Ok::<_, EditorError>(())
        };
        
        // Forward messages from editor server to client
        let server_to_client = async {
            while let Some(server_message) = editor_stream.next().await {
                let json = serde_json::to_string(&server_message)?;
                sender.send(Message::text(json)).await?;
            }
            Ok::<_, EditorError>(())
        };
        
        // Run both directions concurrently
        let _ = tokio::select! {
            result = client_to_server => result,
            result = server_to_client => result,
        };
        
        // Remove user from session when disconnected
        self.active_user_repository.remove_user_from_session(
            &Uuid::parse_str(&session_id)?, 
            &user_id
        ).await?;
        
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Operation(SlateOperation),
    Cursor(CursorPosition),
    RequestHistory,
    RequestSnapshot,
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    Init {
        document: Value,
        version: i64,
        users: HashMap<String, UserInfo>,
    },
    RemoteOperation {
        user_id: String,
        operation: SlateOperation,
        version: i64,
    },
    Cursor {
        user_id: String,
        position: CursorPosition,
    },
    UserJoined {
        user_id: String,
        info: UserInfo,
    },
    UserLeft {
        user_id: String,
    },
    Error {
        code: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub display_name: String,
    pub color: String,
}
```

### 7. AI-Assisted Editing

```rust
pub struct AIAssistService {
    ai_request_repository: Arc<dyn AIAssistRequestRepository>,
    editor_server: Arc<RwLock<EditorServer>>,
    ai_client: Arc<AIClient>,
}

impl AIAssistService {
    pub async fn create_assist_request(
        &self,
        session_id: &str,
        user_id: Uuid,
        prompt: &str,
        selection: Option<TextSelection>,
    ) -> Result<AIAssistRequest, EditorError> {
        // Get document context
        let document_id = Uuid::parse_str(session_id)?;
        let context = self.get_document_context(session_id, &selection).await?;
        
        // Create request
        let request = AIAssistRequest {
            id: Uuid::new_v4(),
            session_id: Uuid::parse_str(session_id)?,
            user_id,
            document_id,
            prompt: prompt.to_string(),
            context,
            selection,
            created_at: Utc::now(),
            status: AIRequestStatus::Pending,
            result: None,
            completed_at: None,
        };
        
        let created_request = self.ai_request_repository.create(request).await?;
        
        // Process request asynchronously
        tokio::spawn(self.process_assist_request(created_request.id));
        
        Ok(created_request)
    }
    
    async fn process_assist_request(&self, request_id: Uuid) {
        let result = async {
            // Get request details
            let mut request = self.ai_request_repository.find_by_id(&request_id).await?;
            
            // Update status to processing
            request.status = AIRequestStatus::Processing;
            self.ai_request_repository.update(request.clone()).await?;
            
            // Send to AI service
            let ai_response = self.ai_client.generate_text(
                &request.prompt,
                &request.context,
                AIParameters::default(),
            ).await?;
            
            // Update request with result
            request.status = AIRequestStatus::Completed;
            request.result = Some(ai_response);
            request.completed_at = Some(Utc::now());
            
            self.ai_request_repository.update(request).await?;
            
            Ok::<_, EditorError>(())
        }.await;
        
        if let Err(e) = result {
            // Update request as failed
            if let Ok(mut request) = self.ai_request_repository.find_by_id(&request_id).await {
                request.status = AIRequestStatus::Failed;
                let _ = self.ai_request_repository.update(request).await;
            }
            
            // Log error
            log::error!("Error processing AI assist request {}: {:?}", request_id, e);
        }
    }
    
    async fn get_document_context(
        &self,
        session_id: &str,
        selection: &Option<TextSelection>,
    ) -> Result<String, EditorError> {
        let editor_server = self.editor_server.read().await;
        let document = editor_server.get_document(session_id)?;
        
        // If selection is provided, extract the selected text
        if let Some(selection) = selection {
            // Extract text from the selected region
            // This is a simplified implementation
            let extracted_text = document.extract_text(
                &selection.start_path,
                selection.start_offset,
                &selection.end_path,
                selection.end_offset,
            )?;
            
            Ok(extracted_text)
        } else {
            // Use the entire document
            Ok(document.to_plain_text()?)
        }
    }
}
```

### 8. Document Export

```rust
pub struct ExportService {
    editor_server: Arc<RwLock<EditorServer>>,
    snapshot_repository: Arc<dyn SnapshotRepository>,
}

impl ExportService {
    pub async fn export_document(
        &self,
        session_id: &str,
        format: ExportFormat,
        options: ExportOptions,
    ) -> Result<Vec<u8>, EditorError> {
        // Get the document from the editor server or from the latest snapshot
        let document = {
            let editor_server = self.editor_server.read().await;
            match editor_server.get_document(session_id) {
                Ok(doc) => doc,
                Err(_) => {
                    // Session might be closed, try to get from snapshot
                    let document_id = Uuid::parse_str(session_id)?;
                    let snapshot = self.snapshot_repository.get_latest_by_document_id(&document_id).await?;
                    SlateDocument::from_json(&snapshot.content)?
                }
            }
        };
        
        // Convert document to requested format
        match format {
            ExportFormat::PlainText => self.export_as_plain_text(&document),
            ExportFormat::Markdown => self.export_as_markdown(&document),
            ExportFormat::HTML => self.export_as_html(&document, &options),
            ExportFormat::DOCX => self.export_as_docx(&document, &options),
            ExportFormat::PDF => self.export_as_pdf(&document, &options),
        }
    }
    
    fn export_as_plain_text(&self, document: &SlateDocument) -> Result<Vec<u8>, EditorError> {
        // Convert structured document to plain text
        let plain_text = document.to_plain_text()?;
        Ok(plain_text.into_bytes())
    }
    
    fn export_as_markdown(&self, document: &SlateDocument) -> Result<Vec<u8>, EditorError> {
        // Convert structured document to markdown
        let markdown = document.to_markdown()?;
        Ok(markdown.into_bytes())
    }
    
    fn export_as_html(&self, document: &SlateDocument, options: &ExportOptions) -> Result<Vec<u8>, EditorError> {
        // Convert structured document to HTML
        let html = document.to_html(options)?;
        Ok(html.into_bytes())
    }
    
    fn export_as_docx(&self, document: &SlateDocument, options: &ExportOptions) -> Result<Vec<u8>, EditorError> {
        // Convert structured document to DOCX
        // This might use pandoc or a specialized library
        todo!("Implement DOCX export")
    }
    
    fn export_as_pdf(&self, document: &SlateDocument, options: &ExportOptions) -> Result<Vec<u8>, EditorError> {
        // Convert structured document to PDF
        // This might use wkhtmltopdf, puppeteer, or a specialized library
        todo!("Implement PDF export")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    PlainText,
    Markdown,
    HTML,
    DOCX,
    PDF,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOptions {
    pub include_comments: bool,
    pub include_history: bool,
    pub stylesheet: Option<String>,
    pub page_size: Option<String>,
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub line_spacing: Option<f32>,
    pub include_table_of_contents: bool,
    pub include_page_numbers: bool,
    pub include_header: bool,
    pub include_footer: bool,
    pub custom_options: HashMap<String, String>,
}
```

### 9. Frontend Integration with Slate.js

The frontend will use Slate.js as the rich text editor framework, integrated with our backend:

```typescript
// Frontend code in TypeScript

// Define custom Slate editor with collaboration features
const CollaborativeEditor = () => {
  const editor = useMemo(() => withHistory(withReact(createEditor())), []);
  const [value, setValue] = useState<Node[]>(initialValue);
  const [users, setUsers] = useState<Record<string, UserInfo>>({});
  const [connected, setConnected] = useState(false);
  
  // Reference to WebSocket connection
  const socketRef = useRef<WebSocket | null>(null);
  
  // Initialize connection when component mounts
  useEffect(() => {
    const sessionId = getSessionIdFromUrl();
    const userId = getCurrentUserId();
    
    // Connect to editor WebSocket
    const socket = new WebSocket(`wss://api.authorworks.io/v1/editor/sessions/${sessionId}/ws`);
    socketRef.current = socket;
    
    socket.addEventListener('open', () => {
      setConnected(true);
    });
    
    socket.addEventListener('message', (event) => {
      const message = JSON.parse(event.data) as ServerMessage;
      
      switch (message.type) {
        case 'init':
          // Initialize editor with document content, version, and user information
          setValue(message.document.children);
          setUsers(message.users);
          break;
          
        case 'remote_operation':
          // Apply remote operation to editor
          const op = message.operation;
          applySlateOperation(editor, op);
          break;
          
        case 'cursor':
          // Update cursor position for a user
          setUsers(prev => ({
            ...prev,
            [message.user_id]: {
              ...prev[message.user_id],
              cursor: message.position
            }
          }));
          break;
          
        case 'user_joined':
          // Add new user to list
          setUsers(prev => ({
            ...prev,
            [message.user_id]: message.info
          }));
          break;
          
        case 'user_left':
          // Remove user from list
          setUsers(prev => {
            const newUsers = { ...prev };
            delete newUsers[message.user_id];
            return newUsers;
          });
          break;
      }
    });
    
    // Clean up WebSocket connection on unmount
    return () => {
      socket.close();
    };
  }, [editor]);
  
  // Send operation to server when editor changes
  const onEditorChange = (newValue: Node[]) => {
    // Calculate operations based on difference between old and new value
    const ops = calculateOperations(value, newValue);
    
    // Send each operation to server
    if (ops.length > 0 && socketRef.current && socketRef.current.readyState === WebSocket.OPEN) {
      ops.forEach(op => {
        const message: ClientMessage = {
          type: 'operation',
          operation: op
        };
        socketRef.current!.send(JSON.stringify(message));
      });
    }
    
    setValue(newValue);
  };
  
  // Send cursor position to server when cursor moves
  const onSelectionChange = (selection: Range | null) => {
    if (selection && socketRef.current && socketRef.current.readyState === WebSocket.OPEN) {
      const position = selectionToPosition(selection);
      const message: ClientMessage = {
        type: 'cursor',
        position
      };
      socketRef.current.send(JSON.stringify(message));
    }
  };
  
  // Render editor with collaboration UI
  return (
    <div className="collaborative-editor">
      <div className="editor-header">
        <ConnectionStatus connected={connected} />
        <ActiveUsers users={users} />
      </div>
      
      <Slate editor={editor} value={value} onChange={onEditorChange}>
        <Toolbar />
        <Editable 
          onSelectionChange={onSelectionChange}
          renderElement={renderElement}
          renderLeaf={renderLeaf}
          renderDecoration={renderUserCursors(users)}
        />
      </Slate>
    </div>
  );
};
```

## Implementation Steps

1. Set up project structure with rustpad as a foundation
2. Implement custom document model for Slate.js compatibility
3. Create operational transformation functions for rich text
4. Build WebSocket server for real-time collaboration
5. Integrate with Content Service for document persistence
6. Add snapshot and versioning functionality
7. Implement AI-assisted editing features
8. Create document export functionality
9. Develop frontend with Slate.js
10. Add user settings and preferences
11. Implement offline editing support with synchronization
12. Create comprehensive tests for all components

## Technical Decisions

### Why Rustpad?

Rustpad was chosen as the foundation for our collaborative editor because:
- It's built in Rust, aligning with our technology stack
- Provides an efficient implementation of operational transformation
- Offers a clean architecture that can be extended for our needs
- Has a minimal, well-designed API
- Includes WebSocket-based collaboration out of the box
- Open source with MIT license

### Why Slate.js?

Slate.js was chosen for the frontend editor because:
- Provides a customizable, framework-agnostic rich text editing library
- Uses an intuitive tree-based document model
- Offers a React-based implementation
- Supports complex formatting and nested document structures
- Has good community support and documentation
- Designed to be extensible for collaborative editing

### Operational Transformation vs. CRDT

Operational Transformation (OT) was chosen over Conflict-free Replicated Data Types (CRDT) because:
- Rustpad already provides an efficient OT implementation
- OT is well-suited for rich text editing
- Lower implementation complexity for our specific use case
- Better performance for small to medium documents
- More predictable behavior for rich text operations

## Success Criteria

The Editor Service will be considered successfully implemented when:

1. Multiple users can simultaneously edit documents with no conflicts
2. Changes are propagated in real-time to all connected clients
3. Document history and versioning work correctly
4. Rich text formatting is properly maintained during collaborative editing
5. AI-assisted editing provides useful suggestions
6. Documents can be exported in various formats
7. The editor handles large documents efficiently
8. Offline editing and synchronization work reliably
9. The system is resilient to network issues and reconnections
10. User preferences are properly applied and persisted 