//! IDE controller
//!
//! The IDE controller expose functionality bound to the application as a whole, not to specific
//! component or opened project.

use crate::prelude::*;

use double_representation::name::project;
use mockall::automock;
use parser_scala::Parser;


// ==============
// === Export ===
// ==============

pub mod desktop;
pub mod plain;

pub use engine_protocol::project_manager::ProjectMetadata;
pub use engine_protocol::project_manager::ProjectName;



// ============================
// === Status Notifications ===
// ============================

/// The handle used to pair the ProcessStarted and ProcessFinished notifications.
pub type BackgroundTaskHandle = usize;

/// A notification which should be displayed to the User on the status bar.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub enum StatusNotification {
    /// Notification about single event, should be logged in an event log window.
    Event { label: String },
    /// Notification about new background task done in IDE (like compiling library).
    BackgroundTaskStarted { label: String, handle: BackgroundTaskHandle },
    /// Notification that some task notified in [`BackgroundTaskStarted`] has been finished.
    BackgroundTaskFinished { handle: BackgroundTaskHandle },
}

/// A publisher for status notification events.
#[derive(Clone, CloneRef, Debug, Default)]
pub struct StatusNotificationPublisher {
    publisher:           notification::Publisher<StatusNotification>,
    next_process_handle: Rc<Cell<usize>>,
}

impl StatusNotificationPublisher {
    /// Constructor.
    pub fn new() -> Self {
        default()
    }

    /// Publish a new status event (see [`StatusNotification::Event`])
    #[profile(Debug)]
    pub fn publish_event(&self, label: impl Into<String>) {
        let label = label.into();
        let notification = StatusNotification::Event { label };
        executor::global::spawn(self.publisher.publish(notification));
    }

    /// Publish a notification about new process (see [`StatusNotification::ProcessStarted`]).
    ///
    /// Returns the handle to be used when notifying about process finishing.
    #[profile(Debug)]
    pub fn publish_background_task(&self, label: impl Into<String>) -> BackgroundTaskHandle {
        let label = label.into();
        let handle = self.next_process_handle.get();
        self.next_process_handle.set(handle + 1);
        let notification = StatusNotification::BackgroundTaskStarted { label, handle };
        executor::global::spawn(self.publisher.publish(notification));
        handle
    }

    /// Publish a notfication that process has finished (see
    /// [`StatusNotification::ProcessFinished`])
    #[profile(Debug)]
    pub fn published_background_task_finished(&self, handle: BackgroundTaskHandle) {
        let notification = StatusNotification::BackgroundTaskFinished { handle };
        executor::global::spawn(self.publisher.publish(notification));
    }

    /// The asynchronous stream of published notifications.
    pub fn subscribe(&self) -> impl Stream<Item = StatusNotification> {
        self.publisher.subscribe()
    }
}



// ====================
// === Notification ===
// ====================

/// Notification of IDE Controller.
///
/// In contrast to [`StatusNotification`], which is a notification from any application part to
/// be delivered to User (displayed on some event log or status bar), this is a notification to be
/// used internally in code.
#[derive(Copy, Clone, Debug)]
pub enum Notification {
    /// User created a new project. The new project is opened in IDE.
    NewProjectCreated,
    /// User opened an existing project.
    ProjectOpened,
}



// ===========
// === API ===
// ===========

// === Errors ===

#[allow(missing_docs)]
#[derive(Clone, Debug, Fail)]
#[fail(display = "Project with name \"{}\" not found.", 0)]
struct ProjectNotFound(String);


// === Managing API ===

/// The API of all project management operations.
///
/// It is a separate trait, because those methods  are not supported in some environments (see also
/// [`API::manage_projects`]).
pub trait ManagingProjectAPI {
    /// Create a new unnamed project and open it in the IDE.
    ///
    /// `template` is an optional project template name. Available template names are defined in
    /// `lib/scala/pkg/src/main/scala/org/enso/pkg/Template.scala`.
    fn create_new_project(&self, template: Option<project::Template>) -> BoxFuture<FallibleResult>;

    /// Return a list of existing projects.
    fn list_projects(&self) -> BoxFuture<FallibleResult<Vec<ProjectMetadata>>>;

    /// Open the project with given UUID.
    fn open_project(&self, id: Uuid) -> BoxFuture<FallibleResult>;

    /// Open project by name. It makes two calls to the Project Manager: one for listing projects
    /// and then for the project opening.
    fn open_project_by_name(&self, name: String) -> BoxFuture<FallibleResult> {
        async move {
            let projects = self.list_projects().await?;
            let mut projects = projects.into_iter();
            let project = projects.find(|project| project.name.as_ref() == name);
            let uuid = project.map(|project| project.id);
            if let Some(uuid) = uuid {
                self.open_project(uuid).await
            } else {
                Err(ProjectNotFound(name).into())
            }
        }
        .boxed_local()
    }
}


// === Main API ===

/// The API of IDE Controller.
#[automock]
pub trait API: Debug {
    /// The model of currently opened project.
    ///
    /// IDE can have only one project opened at a time.
    ///
    /// Returns `None` if no project is opened at the moment.
    fn current_project(&self) -> Option<model::Project>;

    /// Getter of Status Notification Publisher.
    fn status_notifications(&self) -> &StatusNotificationPublisher;

    /// The Parser Handle.
    fn parser(&self) -> &Parser;

    /// Subscribe the controller notifications.
    fn subscribe(&self) -> StaticBoxStream<Notification>;

    /// Return the Managing Project API.
    ///
    /// It may be some delegated object or just the reference to self.
    // Automock macro does not work without explicit lifetimes here.
    #[allow(clippy::needless_lifetimes)]
    fn manage_projects<'a>(&'a self) -> FallibleResult<&'a dyn ManagingProjectAPI>;
}

/// A polymorphic handle of IDE controller.
pub type Ide = Rc<dyn API>;

/// The IDE Controller for desktop environments.
pub type Desktop = desktop::Handle;

/// The Plain IDE controller with a single project and no possibility to change it.
pub type Plain = plain::Handle;

impl Debug for MockAPI {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mocked Ide Controller")
    }
}
