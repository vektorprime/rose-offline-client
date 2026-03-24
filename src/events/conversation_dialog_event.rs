use bevy::prelude::{Entity, Message};

use rose_file_readers::VfsPathBuf;

#[derive(Message)]
pub enum ConversationDialogEvent {
    OpenNpcDialog(Entity, VfsPathBuf),
    OpenEventDialog(VfsPathBuf),
}
