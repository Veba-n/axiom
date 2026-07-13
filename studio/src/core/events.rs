#[derive(Debug, Clone, PartialEq)]
pub enum AxiomEvent {
    ActionTriggered(String),
    SceneChangeRequested(String),
    PushScene(String),
    PopScene,
    SystemMessage(String),
}
