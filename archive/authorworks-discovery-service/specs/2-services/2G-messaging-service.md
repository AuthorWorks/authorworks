# Technical Specification: 2G - Messaging Service

## Overview

The Messaging Service provides real-time communication capabilities for the AuthorWorks platform. It enables users to send direct messages, participate in group conversations, receive notifications, and collaborate in real-time. This service leverages Matrix as the underlying protocol for federation and implements a gateway to simplify client interactions with the Matrix ecosystem.

## Objectives

- Enable direct messaging between users
- Support group chat functionality for collaboration
- Provide real-time notifications for platform events
- Implement typing indicators and read receipts
- Support media sharing within conversations
- Enable message threading and reactions
- Integrate with user authentication and permissions

## Requirements

### 1. Core Messaging Data Models

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: Uuid,
    pub conversation_type: ConversationType,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub matrix_room_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConversationType {
    DirectMessage,
    Group,
    ProjectCollaboration,
    SystemNotification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMember {
    pub conversation_id: Uuid,
    pub user_id: Uuid,
    pub role: MemberRole,
    pub joined_at: DateTime<Utc>,
    pub last_read_at: Option<DateTime<Utc>>,
    pub matrix_user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemberRole {
    Owner,
    Admin,
    Member,
    Guest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub content_type: MessageContentType,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub parent_id: Option<Uuid>,
    pub matrix_event_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContentType {
    Text,
    Image,
    File,
    Audio,
    Video,
    SystemNotification,
    CollaborationUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageReaction {
    pub message_id: Uuid,
    pub user_id: Uuid,
    pub reaction: String,
    pub created_at: DateTime<Utc>,
    pub matrix_event_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreference {
    pub user_id: Uuid,
    pub conversation_id: Option<Uuid>,
    pub notification_type: NotificationType,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    DirectMessages,
    GroupMessages,
    Mentions,
    CollaborationUpdates,
    SystemNotifications,
}
```

### 2. Database Schema

```sql
CREATE TABLE conversations (
    id UUID PRIMARY KEY,
    conversation_type VARCHAR(30) NOT NULL,
    title VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    last_message_at TIMESTAMP WITH TIME ZONE,
    matrix_room_id VARCHAR(255) NOT NULL UNIQUE
);

CREATE TABLE conversation_members (
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),
    role VARCHAR(20) NOT NULL,
    joined_at TIMESTAMP WITH TIME ZONE NOT NULL,
    last_read_at TIMESTAMP WITH TIME ZONE,
    matrix_user_id VARCHAR(255) NOT NULL,
    PRIMARY KEY (conversation_id, user_id)
);

CREATE TABLE messages (
    id UUID PRIMARY KEY,
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    sender_id UUID NOT NULL REFERENCES users(id),
    content_type VARCHAR(30) NOT NULL,
    content TEXT NOT NULL,
    metadata JSONB,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    edited_at TIMESTAMP WITH TIME ZONE,
    parent_id UUID REFERENCES messages(id),
    matrix_event_id VARCHAR(255) NOT NULL UNIQUE
);

CREATE TABLE message_reactions (
    message_id UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),
    reaction VARCHAR(50) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    matrix_event_id VARCHAR(255) NOT NULL UNIQUE,
    PRIMARY KEY (message_id, user_id, reaction)
);

CREATE TABLE notification_preferences (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    conversation_id UUID REFERENCES conversations(id) ON DELETE CASCADE,
    notification_type VARCHAR(30) NOT NULL,
    enabled BOOLEAN NOT NULL,
    PRIMARY KEY (user_id, conversation_id, notification_type)
);

-- Indexes
CREATE INDEX idx_conversation_last_message ON conversations(last_message_at);
CREATE INDEX idx_messages_conversation ON messages(conversation_id);
CREATE INDEX idx_messages_sender ON messages(sender_id);
CREATE INDEX idx_messages_parent ON messages(parent_id) WHERE parent_id IS NOT NULL;
CREATE INDEX idx_messages_created_at ON messages(created_at);
CREATE INDEX idx_conversation_members_user ON conversation_members(user_id);
```

### 3. API Endpoints

```
# Conversation Management
POST   /v1/conversations                           - Create new conversation
GET    /v1/conversations                           - List user's conversations
GET    /v1/conversations/{id}                      - Get conversation details
DELETE /v1/conversations/{id}                      - Delete conversation
PUT    /v1/conversations/{id}                      - Update conversation details

# Conversation Membership
GET    /v1/conversations/{id}/members              - List conversation members
POST   /v1/conversations/{id}/members              - Add member to conversation
DELETE /v1/conversations/{id}/members/{userId}     - Remove member from conversation
PUT    /v1/conversations/{id}/members/{userId}/role - Update member role

# Messages
POST   /v1/conversations/{id}/messages             - Send message
GET    /v1/conversations/{id}/messages             - Get conversation messages
PUT    /v1/conversations/{id}/messages/{messageId} - Edit message
DELETE /v1/conversations/{id}/messages/{messageId} - Delete message
POST   /v1/conversations/{id}/typing               - Send typing indicator

# Message Threading
GET    /v1/messages/{messageId}/thread             - Get message thread
POST   /v1/messages/{messageId}/thread             - Reply in thread

# Reactions
POST   /v1/messages/{messageId}/reactions          - Add reaction
DELETE /v1/messages/{messageId}/reactions/{reaction} - Remove reaction
GET    /v1/messages/{messageId}/reactions          - List reactions

# Read Receipts
POST   /v1/conversations/{id}/read                 - Mark conversation as read
GET    /v1/conversations/{id}/read-status          - Get read status for conversation

# Notifications
GET    /v1/notifications                           - Get recent notifications
PUT    /v1/notifications/preferences               - Update notification preferences
GET    /v1/notifications/preferences               - Get notification preferences

# WebSocket endpoint for real-time updates
WS     /v1/messaging/ws                            - WebSocket connection
```

### 4. Matrix Integration

The Messaging Service acts as a gateway between the AuthorWorks platform and Matrix network. It provides an abstraction layer that simplifies client interactions and provides a domain-specific API for the AuthorWorks platform.

```rust
pub struct MatrixClient {
    homeserver_url: String,
    client: Client,
    session: Arc<RwLock<Option<Session>>>,
}

impl MatrixClient {
    pub async fn new(homeserver_url: String) -> Result<Self, MatrixError> {
        let client = Client::new(homeserver_url.clone())?;
        
        Ok(Self {
            homeserver_url,
            client,
            session: Arc::new(RwLock::new(None)),
        })
    }
    
    pub async fn login(&self, username: &str, password: &str) -> Result<(), MatrixError> {
        let response = self.client.login(username, password, None, Some("AuthorWorks Messaging Service")).await?;
        
        let mut session_guard = self.session.write().await;
        *session_guard = Some(Session {
            access_token: response.access_token,
            user_id: response.user_id,
            device_id: response.device_id,
        });
        
        Ok(())
    }
    
    pub async fn create_room(&self, name: Option<&str>, is_direct: bool, invited_user_ids: &[&str]) -> Result<String, MatrixError> {
        self.ensure_logged_in().await?;
        
        let mut request = RoomCreationRequest::new();
        
        if let Some(name) = name {
            request.name = Some(name.to_string());
        }
        
        request.is_direct = Some(is_direct);
        request.invite = Some(invited_user_ids.iter().map(|&id| id.to_string()).collect());
        request.preset = Some(if is_direct { 
            RoomPreset::TrustedPrivateChat 
        } else { 
            RoomPreset::PrivateChat 
        });
        
        let response = self.client.create_room(request).await?;
        Ok(response.room_id)
    }
    
    pub async fn send_message(&self, room_id: &str, content: MessageContent) -> Result<String, MatrixError> {
        self.ensure_logged_in().await?;
        
        let txn_id = Uuid::new_v4().to_string();
        let event_id = match content {
            MessageContent::Text(text) => {
                let content = RoomMessageEventContent::text_plain(text);
                self.client.room_send(room_id, &txn_id, &content).await?
            },
            MessageContent::Image { url, info } => {
                let content = RoomMessageEventContent::new(MessageType::Image(ImageMessageEventContent::new(
                    url,
                    info.map(|i| ImageInfo {
                        height: i.height,
                        width: i.width,
                        mimetype: i.mimetype,
                        size: i.size,
                        thumbnail_info: None,
                        thumbnail_url: None,
                        blurhash: None,
                    })
                )));
                self.client.room_send(room_id, &txn_id, &content).await?
            },
            // Add other message types as needed
        };
        
        Ok(event_id)
    }
    
    pub async fn get_messages(&self, room_id: &str, from: Option<&str>, limit: Option<u32>) -> Result<Vec<RoomEvent>, MatrixError> {
        self.ensure_logged_in().await?;
        
        let response = self.client.room_messages(
            room_id,
            from.unwrap_or(""),
            None,
            Direction::Backward,
            Some(limit.unwrap_or(50)),
            None,
        ).await?;
        
        Ok(response.chunk)
    }
    
    // Additional Matrix API methods
    // ...
    
    async fn ensure_logged_in(&self) -> Result<(), MatrixError> {
        let session = self.session.read().await;
        
        if session.is_none() {
            return Err(MatrixError::NotLoggedIn);
        }
        
        Ok(())
    }
}

pub enum MessageContent {
    Text(String),
    Image { url: String, info: Option<ImageInfo> },
    File { url: String, info: Option<FileInfo> },
    Audio { url: String, info: Option<AudioInfo> },
    Video { url: String, info: Option<VideoInfo> },
}

pub struct ImageInfo {
    pub height: u32,
    pub width: u32,
    pub mimetype: String,
    pub size: u64,
}

// Define other info structs for different content types
```

### 5. Conversation Service

The Conversation Service handles the creation, management, and retrieval of conversations.

```rust
pub struct ConversationService {
    conversation_repository: Arc<dyn ConversationRepository>,
    member_repository: Arc<dyn MemberRepository>,
    user_service: Arc<dyn UserService>,
    matrix_client: Arc<MatrixClient>,
}

impl ConversationService {
    pub async fn create_direct_conversation(&self, user_id: Uuid, other_user_id: Uuid) -> Result<Conversation, MessagingError> {
        // Check if direct conversation already exists
        if let Some(conversation) = self.find_direct_conversation(user_id, other_user_id).await? {
            return Ok(conversation);
        }
        
        // Get matrix user IDs
        let current_user = self.user_service.get_user(user_id).await?;
        let other_user = self.user_service.get_user(other_user_id).await?;
        
        let matrix_user_id = current_user.matrix_user_id.ok_or(MessagingError::MatrixUserIdNotFound)?;
        let other_matrix_user_id = other_user.matrix_user_id.ok_or(MessagingError::MatrixUserIdNotFound)?;
        
        // Create Matrix room
        let room_id = self.matrix_client.create_room(
            None,
            true,
            &[&other_matrix_user_id],
        ).await?;
        
        // Create conversation
        let conversation = Conversation {
            id: Uuid::new_v4(),
            conversation_type: ConversationType::DirectMessage,
            title: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_message_at: None,
            matrix_room_id: room_id,
        };
        
        let created_conversation = self.conversation_repository.create(conversation).await?;
        
        // Add members
        let creator_member = ConversationMember {
            conversation_id: created_conversation.id,
            user_id,
            role: MemberRole::Member,
            joined_at: Utc::now(),
            last_read_at: None,
            matrix_user_id: matrix_user_id,
        };
        
        let other_member = ConversationMember {
            conversation_id: created_conversation.id,
            user_id: other_user_id,
            role: MemberRole::Member,
            joined_at: Utc::now(),
            last_read_at: None,
            matrix_user_id: other_matrix_user_id,
        };
        
        self.member_repository.create(creator_member).await?;
        self.member_repository.create(other_member).await?;
        
        Ok(created_conversation)
    }
    
    pub async fn create_group_conversation(
        &self,
        creator_id: Uuid,
        title: String,
        member_ids: Vec<Uuid>,
    ) -> Result<Conversation, MessagingError> {
        if title.is_empty() {
            return Err(MessagingError::InvalidTitle);
        }
        
        if member_ids.is_empty() {
            return Err(MessagingError::NoMembers);
        }
        
        // Get creator matrix user ID
        let creator = self.user_service.get_user(creator_id).await?;
        let creator_matrix_id = creator.matrix_user_id.ok_or(MessagingError::MatrixUserIdNotFound)?;
        
        // Get matrix user IDs for all members
        let mut matrix_member_ids = Vec::new();
        let mut all_members = Vec::new();
        
        for member_id in &member_ids {
            let member = self.user_service.get_user(*member_id).await?;
            let matrix_id = member.matrix_user_id.ok_or(MessagingError::MatrixUserIdNotFound)?;
            
            matrix_member_ids.push(matrix_id.clone());
            all_members.push(ConversationMember {
                conversation_id: Uuid::nil(), // Placeholder, will be updated
                user_id: *member_id,
                role: MemberRole::Member,
                joined_at: Utc::now(),
                last_read_at: None,
                matrix_user_id: matrix_id,
            });
        }
        
        // Create Matrix room
        let room_id = self.matrix_client.create_room(
            Some(&title),
            false,
            &matrix_member_ids.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
        ).await?;
        
        // Create conversation
        let conversation = Conversation {
            id: Uuid::new_v4(),
            conversation_type: ConversationType::Group,
            title: Some(title),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_message_at: None,
            matrix_room_id: room_id,
        };
        
        let created_conversation = self.conversation_repository.create(conversation).await?;
        
        // Add creator as owner
        let creator_member = ConversationMember {
            conversation_id: created_conversation.id,
            user_id: creator_id,
            role: MemberRole::Owner,
            joined_at: Utc::now(),
            last_read_at: None,
            matrix_user_id: creator_matrix_id,
        };
        
        self.member_repository.create(creator_member).await?;
        
        // Add members
        for mut member in all_members {
            member.conversation_id = created_conversation.id;
            self.member_repository.create(member).await?;
        }
        
        Ok(created_conversation)
    }
    
    pub async fn get_conversation(&self, conversation_id: Uuid) -> Result<Conversation, MessagingError> {
        self.conversation_repository.find_by_id(conversation_id)
            .await?
            .ok_or(MessagingError::ConversationNotFound)
    }
    
    pub async fn get_user_conversations(&self, user_id: Uuid) -> Result<Vec<Conversation>, MessagingError> {
        self.conversation_repository.find_by_user_id(user_id).await
    }
    
    pub async fn find_direct_conversation(&self, user_id: Uuid, other_user_id: Uuid) -> Result<Option<Conversation>, MessagingError> {
        self.conversation_repository.find_direct_conversation(user_id, other_user_id).await
    }
    
    // Additional methods
    // ...
}
```

### 6. Message Service

The Message Service handles sending, retrieving, and managing messages in conversations.

```rust
pub struct MessageService {
    message_repository: Arc<dyn MessageRepository>,
    conversation_repository: Arc<dyn ConversationRepository>,
    member_repository: Arc<dyn MemberRepository>,
    matrix_client: Arc<MatrixClient>,
}

impl MessageService {
    pub async fn send_message(
        &self,
        user_id: Uuid,
        conversation_id: Uuid,
        content_type: MessageContentType,
        content: String,
        parent_id: Option<Uuid>,
    ) -> Result<Message, MessagingError> {
        // Verify user is a member of the conversation
        let member = self.member_repository.find_by_user_id_and_conversation_id(user_id, conversation_id)
            .await?
            .ok_or(MessagingError::NotConversationMember)?;
        
        // Get conversation
        let conversation = self.conversation_repository.find_by_id(conversation_id)
            .await?
            .ok_or(MessagingError::ConversationNotFound)?;
        
        // Verify parent message if specified
        if let Some(parent_id) = parent_id {
            self.message_repository.find_by_id(parent_id)
                .await?
                .ok_or(MessagingError::ParentMessageNotFound)?;
        }
        
        // Convert to Matrix message content
        let matrix_content = match content_type {
            MessageContentType::Text => MessageContent::Text(content.clone()),
            MessageContentType::Image => {
                // Parse metadata from content
                let metadata: ImageMetadata = serde_json::from_str(&content)?;
                
                MessageContent::Image {
                    url: metadata.url,
                    info: Some(ImageInfo {
                        height: metadata.height,
                        width: metadata.width,
                        mimetype: metadata.mimetype,
                        size: metadata.size,
                    }),
                }
            },
            // Handle other content types
            _ => return Err(MessagingError::UnsupportedContentType),
        };
        
        // Send message to Matrix
        let event_id = self.matrix_client.send_message(
            &conversation.matrix_room_id,
            matrix_content,
        ).await?;
        
        // Create message
        let message = Message {
            id: Uuid::new_v4(),
            conversation_id,
            sender_id: user_id,
            content_type,
            content,
            metadata: None,
            created_at: Utc::now(),
            edited_at: None,
            parent_id,
            matrix_event_id: event_id,
        };
        
        let created_message = self.message_repository.create(message).await?;
        
        // Update conversation last_message_at
        self.conversation_repository.update_last_message_time(conversation_id, created_message.created_at).await?;
        
        Ok(created_message)
    }
    
    pub async fn get_conversation_messages(
        &self,
        user_id: Uuid,
        conversation_id: Uuid,
        limit: usize,
        before: Option<DateTime<Utc>>,
    ) -> Result<Vec<Message>, MessagingError> {
        // Verify user is a member of the conversation
        self.member_repository.find_by_user_id_and_conversation_id(user_id, conversation_id)
            .await?
            .ok_or(MessagingError::NotConversationMember)?;
        
        // Get messages
        self.message_repository.find_by_conversation_id(
            conversation_id,
            limit,
            before,
        ).await
    }
    
    pub async fn get_message_thread(
        &self,
        user_id: Uuid,
        message_id: Uuid,
        limit: usize,
        before: Option<DateTime<Utc>>,
    ) -> Result<Vec<Message>, MessagingError> {
        // Get parent message
        let parent_message = self.message_repository.find_by_id(message_id)
            .await?
            .ok_or(MessagingError::MessageNotFound)?;
        
        // Verify user is a member of the conversation
        self.member_repository.find_by_user_id_and_conversation_id(user_id, parent_message.conversation_id)
            .await?
            .ok_or(MessagingError::NotConversationMember)?;
        
        // Get thread messages
        self.message_repository.find_by_parent_id(
            message_id,
            limit,
            before,
        ).await
    }
    
    pub async fn add_reaction(
        &self,
        user_id: Uuid,
        message_id: Uuid,
        reaction: String,
    ) -> Result<MessageReaction, MessagingError> {
        // Get message
        let message = self.message_repository.find_by_id(message_id)
            .await?
            .ok_or(MessagingError::MessageNotFound)?;
        
        // Verify user is a member of the conversation
        self.member_repository.find_by_user_id_and_conversation_id(user_id, message.conversation_id)
            .await?
            .ok_or(MessagingError::NotConversationMember)?;
        
        // Get conversation
        let conversation = self.conversation_repository.find_by_id(message.conversation_id)
            .await?
            .ok_or(MessagingError::ConversationNotFound)?;
        
        // Send reaction to Matrix
        let event_id = self.matrix_client.send_reaction(
            &conversation.matrix_room_id,
            &message.matrix_event_id,
            &reaction,
        ).await?;
        
        // Create reaction
        let message_reaction = MessageReaction {
            message_id,
            user_id,
            reaction,
            created_at: Utc::now(),
            matrix_event_id: event_id,
        };
        
        let created_reaction = self.message_repository.create_reaction(message_reaction).await?;
        
        Ok(created_reaction)
    }
    
    // Additional methods
    // ...
}
```

### 7. Real-time Notifications

The Messaging Service implements a WebSocket-based real-time notification system to deliver messages and updates to clients.

```rust
pub struct NotificationService {
    active_connections: Arc<RwLock<HashMap<Uuid, Vec<mpsc::Sender<Message>>>>>,
    message_repository: Arc<dyn MessageRepository>,
    conversation_repository: Arc<dyn ConversationRepository>,
    member_repository: Arc<dyn MemberRepository>,
}

impl NotificationService {
    pub async fn handle_connection(
        &self,
        user_id: Uuid,
        websocket: WebSocket,
    ) -> Result<(), MessagingError> {
        // Split the websocket
        let (sender, mut receiver) = websocket.split();
        
        // Create channel for sending messages to this connection
        let (tx, mut rx) = mpsc::channel::<Message>(100);
        
        // Register connection
        {
            let mut connections = self.active_connections.write().await;
            let user_connections = connections.entry(user_id).or_insert_with(Vec::new);
            user_connections.push(tx);
        }
        
        // Handle incoming messages from the channel
        let send_task = tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if sender.send(message).await.is_err() {
                    break;
                }
            }
        });
        
        // Handle incoming messages from the websocket
        let mut receive_task_finished = false;
        while let Some(result) = receiver.next().await {
            match result {
                Ok(message) => {
                    // Handle client messages
                    // ...
                },
                Err(_) => {
                    // Connection closed
                    receive_task_finished = true;
                    break;
                }
            }
        }
        
        // Remove connection
        {
            let mut connections = self.active_connections.write().await;
            if let Some(user_connections) = connections.get_mut(&user_id) {
                user_connections.retain(|tx| !tx.is_closed());
                if user_connections.is_empty() {
                    connections.remove(&user_id);
                }
            }
        }
        
        // Cancel send task if receive task finished
        if receive_task_finished {
            send_task.abort();
        }
        
        Ok(())
    }
    
    pub async fn notify_new_message(&self, message: &Message) -> Result<(), MessagingError> {
        // Get conversation members
        let members = self.member_repository.find_by_conversation_id(message.conversation_id).await?;
        
        // Send notification to all members except sender
        for member in members {
            if member.user_id == message.sender_id {
                continue;
            }
            
            self.send_notification(
                member.user_id,
                NotificationType::NewMessage,
                serde_json::to_value(message)?,
            ).await?;
        }
        
        Ok(())
    }
    
    pub async fn send_notification(
        &self,
        user_id: Uuid,
        notification_type: NotificationType,
        data: serde_json::Value,
    ) -> Result<(), MessagingError> {
        let notification = Notification {
            type_: notification_type,
            data,
            timestamp: Utc::now(),
        };
        
        let notification_json = serde_json::to_string(&notification)?;
        let message = Message::text(notification_json);
        
        let connections = self.active_connections.read().await;
        if let Some(user_connections) = connections.get(&user_id) {
            for tx in user_connections {
                let _ = tx.send(message.clone()).await;
            }
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    #[serde(rename = "type")]
    pub type_: NotificationType,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    NewMessage,
    MessageEdited,
    MessageDeleted,
    MessageReaction,
    UserTyping,
    ConversationCreated,
    MemberAdded,
    MemberRemoved,
    MemberUpdated,
}
```

### 8. Matrix Synchronization

The Messaging Service implements a background syncing process to keep the local database in sync with Matrix events.

```rust
pub struct MatrixSyncService {
    matrix_client: Arc<MatrixClient>,
    message_repository: Arc<dyn MessageRepository>,
    conversation_repository: Arc<dyn ConversationRepository>,
    member_repository: Arc<dyn MemberRepository>,
    notification_service: Arc<NotificationService>,
}

impl MatrixSyncService {
    pub async fn start_sync(&self) -> Result<(), MessagingError> {
        let mut sync_stream = self.matrix_client.sync_forever(None, None).await?;
        
        while let Some(sync_response) = sync_stream.next().await {
            match sync_response {
                Ok(response) => {
                    self.process_sync_response(response).await?;
                },
                Err(err) => {
                    log::error!("Matrix sync error: {:?}", err);
                    // Implement backoff strategy
                    time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
        
        Ok(())
    }
    
    async fn process_sync_response(&self, response: SyncResponse) -> Result<(), MessagingError> {
        // Process rooms
        for (room_id, room) in response.rooms.join {
            // Process timeline events
            for event in room.timeline.events {
                match event {
                    SyncRoomEvent::Message(event) => {
                        self.process_message_event(room_id.clone(), event).await?;
                    },
                    SyncRoomEvent::Reaction(event) => {
                        self.process_reaction_event(room_id.clone(), event).await?;
                    },
                    // Handle other event types
                    _ => {},
                }
            }
            
            // Process state events
            for event in room.state.events {
                match event {
                    SyncStateEvent::RoomMember(event) => {
                        self.process_member_event(room_id.clone(), event).await?;
                    },
                    // Handle other state events
                    _ => {},
                }
            }
        }
        
        Ok(())
    }
    
    async fn process_message_event(
        &self,
        room_id: String,
        event: MessageEvent,
    ) -> Result<(), MessagingError> {
        // Skip messages sent by the service itself
        if event.sender == self.matrix_client.get_user_id().await? {
            return Ok(());
        }
        
        // Find conversation by Matrix room ID
        let conversation = self.conversation_repository.find_by_matrix_room_id(&room_id)
            .await?
            .ok_or(MessagingError::ConversationNotFound)?;
        
        // Find sender by Matrix user ID
        let sender = self.member_repository.find_by_matrix_user_id_and_conversation_id(
            &event.sender,
            conversation.id,
        ).await?;
        
        // Skip if sender not found
        let sender = match sender {
            Some(s) => s,
            None => return Ok(()),
        };
        
        // Convert message content
        let (content_type, content) = match event.content.msgtype {
            MessageType::Text(text) => (
                MessageContentType::Text,
                text.body,
            ),
            MessageType::Image(image) => (
                MessageContentType::Image,
                serde_json::to_string(&ImageMetadata {
                    url: image.url,
                    height: image.info.as_ref().and_then(|i| i.height).unwrap_or(0),
                    width: image.info.as_ref().and_then(|i| i.width).unwrap_or(0),
                    mimetype: image.info.as_ref().and_then(|i| i.mimetype.clone()).unwrap_or_default(),
                    size: image.info.as_ref().and_then(|i| i.size).unwrap_or(0),
                })?,
            ),
            // Handle other message types
            _ => return Ok(()),
        };
        
        // Create message
        let message = Message {
            id: Uuid::new_v4(),
            conversation_id: conversation.id,
            sender_id: sender.user_id,
            content_type,
            content,
            metadata: None,
            created_at: event.origin_server_ts.into(),
            edited_at: None,
            parent_id: None, // Handle thread replies
            matrix_event_id: event.event_id,
        };
        
        let created_message = self.message_repository.create(message).await?;
        
        // Update conversation last_message_at
        self.conversation_repository.update_last_message_time(
            conversation.id,
            created_message.created_at,
        ).await?;
        
        // Send notification
        self.notification_service.notify_new_message(&created_message).await?;
        
        Ok(())
    }
    
    // Implement other event processing methods
    // ...
}
```

## Implementation Steps

1. Set up project structure with required dependencies
2. Implement database models and repositories
3. Create Matrix client integration
4. Implement conversation service
5. Develop message service
6. Build WebSocket notification system
7. Create Matrix synchronization service
8. Implement API endpoints
9. Add authentication and authorization
10. Write comprehensive tests

## Technical Decisions

### Why Matrix for Messaging?

Matrix was chosen as the underlying protocol for messaging for several reasons:
- Open standard with strong federation capabilities
- End-to-end encryption support
- Mature ecosystem with multiple client and server implementations
- Supports all required features (direct messages, groups, threading, reactions)
- Self-hostable with Synapse server
- Extensive documentation and active development

### Why a Gateway Pattern?

The gateway pattern was chosen for Matrix integration because:
- Simplifies client interaction by providing a domain-specific API
- Centralizes Matrix authentication and communication
- Enables mapping between AuthorWorks entities and Matrix concepts
- Provides a consistent experience across the platform
- Allows for future protocol changes without affecting clients

## Success Criteria

The Messaging Service will be considered successfully implemented when:

1. Users can send and receive direct messages in real-time
2. Group conversations function properly with appropriate permissions
3. Message threading and reactions work as expected
4. Real-time notifications are delivered reliably
5. Matrix synchronization maintains a consistent state
6. Media sharing works correctly across all content types
7. The service handles high volume with low latency
8. All endpoints are properly secured with authentication
9. Comprehensive test coverage validates functionality 