use iced::{Element, widget::{column, row, text, button, text_input, scrollable, container}, Length};
use crate::workflows::{Workflow, WorkflowStep};
use crate::Message;

/// Represents the UI for managing workflows.
#[derive(Debug, Clone)]
pub struct WorkflowUI {
    manager: WorkflowManager,
    search_query: String,
    selected_category: Option<WorkflowCategory>,
    selected_shell: Option<Shell>,
    search_results: Vec<WorkflowSearchResult>,
    pub selected_workflow: Option<Workflow>,
    argument_values: HashMap<String, String>,
    show_workflow_details: bool,
    show_create_workflow: bool,
    new_workflow: Workflow,
    workflow_editor: WorkflowEditor,
    pub new_workflow_name: String,
    pub new_step_command: String,
    pub new_step_name: String,
}

#[derive(Debug, Clone)]
pub enum WorkflowMessage {
    SearchChanged(String),
    CategorySelected(Option<WorkflowCategory>),
    ShellSelected(Option<Shell>),
    WorkflowSelected(String),
    ArgumentChanged(String, String),
    ExecuteWorkflow,
    DryRunWorkflow,
    ShowWorkflowDetails(bool),
    ShowCreateWorkflow(bool),
    CreateWorkflow,
    EditWorkflow(Workflow),
    DeleteWorkflow(String),
    ImportWorkflow(String),
    ExportWorkflow(String),
    RefreshWorkflows,
    NewWorkflowNameChanged(String),
    CreateNewWorkflow,
    StepCommandChanged(String),
    StepNameChanged(String),
    AddStepToWorkflow,
    RemoveStepFromWorkflow(usize),
}

impl WorkflowUI {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let manager = WorkflowManager::new()?;
        let search_results = manager.get_all_workflows(None);
        let workflow_editor = WorkflowEditor::new();

        Ok(Self {
            manager,
            search_query: String::new(),
            selected_category: None,
            selected_shell: None,
            search_results,
            selected_workflow: None,
            argument_values: HashMap::new(),
            show_workflow_details: false,
            show_create_workflow: false,
            new_workflow: Workflow {
                name: String::new(),
                command: String::new(),
                tags: Vec::new(),
                description: None,
                source_url: None,
                author: None,
                author_url: None,
                shells: None,
                arguments: Vec::new(),
                file_path: None,
                last_used: None,
                usage_count: 0,
            },
            workflow_editor,
            new_workflow_name: String::new(),
            new_step_command: String::new(),
            new_step_name: String::new(),
        })
    }

    pub fn update(&mut self, message: WorkflowMessage) {
        match message {
            WorkflowMessage::SearchChanged(query) => {
                self.search_query = query;
                self.update_search_results();
            }
            WorkflowMessage::CategorySelected(category) => {
                self.selected_category = category;
                self.update_search_results();
            }
            WorkflowMessage::ShellSelected(shell) => {
                self.selected_shell = shell;
                self.update_search_results();
            }
            WorkflowMessage::WorkflowSelected(name) => {
                self.selected_workflow = self.workflows.iter().find(|wf| wf.name == name).cloned();
                self.argument_values.clear();
                
                // Initialize argument values with defaults
                if let Some(workflow) = &self.selected_workflow {
                    for arg in &workflow.arguments {
                        if let Some(default) = &arg.default_value {
                            self.argument_values.insert(arg.name.clone(), default.clone());
                        }
                    }
                }
            }
            WorkflowMessage::ArgumentChanged(name, value) => {
                self.argument_values.insert(name, value);
            }
            WorkflowMessage::ExecuteWorkflow => {
                // Handle workflow execution
            }
            WorkflowMessage::DryRunWorkflow => {
                // Handle dry run workflow
            }
            WorkflowMessage::ShowWorkflowDetails(show) => {
                self.show_workflow_details = show;
            }
            WorkflowMessage::RefreshWorkflows => {
                if let Err(e) = self.manager.load_workflows() {
                    eprintln!("Failed to refresh workflows: {}", e);
                }
                self.update_search_results();
            }
            WorkflowMessage::NewWorkflowNameChanged(name) => {
                self.new_workflow_name = name;
            }
            WorkflowMessage::CreateNewWorkflow => {
                if !self.new_workflow_name.is_empty() {
                    let new_workflow = Workflow {
                        name: self.new_workflow_name.clone(),
                        description: "New workflow".to_string(),
                        steps: Vec::new(),
                        environment: HashMap::new(),
                        timeout: None,
                    };
                    self.workflows.push(new_workflow.clone());
                    self.selected_workflow = Some(new_workflow);
                    self.new_workflow_name.clear();
                }
            }
            WorkflowMessage::DeleteWorkflow(name) => {
                self.workflows.retain(|wf| wf.name != name);
                self.selected_workflow = None;
            }
            WorkflowMessage::StepCommandChanged(command) => {
                self.new_step_command = command;
            }
            WorkflowMessage::StepNameChanged(name) => {
                self.new_step_name = name;
            }
            WorkflowMessage::AddStepToWorkflow => {
                if let Some(workflow) = self.selected_workflow.as_mut() {
                    if !self.new_step_name.is_empty() && !self.new_step_command.is_empty() {
                        let new_step = WorkflowStep {
                            name: self.new_step_name.clone(),
                            command: self.new_step_command.clone(),
                            args: Vec::new(),
                            working_directory: None,
                            environment: HashMap::new(),
                            timeout: None,
                            retry_count: 0,
                            condition: None,
                        };
                        workflow.steps.push(new_step);
                        self.new_step_name.clear();
                        self.new_step_command.clear();
                    }
                }
            }
            WorkflowMessage::RemoveStepFromWorkflow(index) => {
                if let Some(workflow) = self.selected_workflow.as_mut() {
                    if index < workflow.steps.len() {
                        workflow.steps.remove(index);
                    }
                }
            }
        }
    }

    fn update_search_results(&mut self) {
        self.search_results = if self.search_query.is_empty() {
            if let Some(category) = &self.selected_category {
                self.manager.get_workflows_by_category(category, self.selected_shell.as_ref())
                    .into_iter()
                    .map(|workflow| WorkflowSearchResult {
                        workflow,
                        score: 0.0,
                        matched_fields: vec![],
                    })
                    .collect()
            } else {
                self.manager.get_all_workflows(self.selected_shell.as_ref())
            }
        } else {
            self.manager.search_workflows(&self.search_query, self.selected_shell.as_ref())
        };
    }

    pub fn view(&self) -> Element<WorkflowMessage> {
        let main_content = column![
            self.create_header(),
            self.create_filters(),
            self.create_workflow_list(),
            if self.selected_workflow.is_some() {
                self.create_workflow_details()
            } else {
                container(text("Select a workflow to see details")).into()
            }
        ]
        .spacing(16)
        .padding(16);

        if self.show_create_workflow {
            self.create_workflow_dialog()
        } else {
            scrollable(main_content).into()
        }
    }

    fn create_header(&self) -> Element<WorkflowMessage> {
        row![
            text("Workflows").size(24),
            // Spacer
            iced::widget::horizontal_space(iced::Length::Fill),
            text_input("Search workflows...", &self.search_query)
                .on_input(WorkflowMessage::SearchChanged)
                .width(iced::Length::Fixed(300.0)),
            button("Refresh")
                .on_press(WorkflowMessage::RefreshWorkflows),
            button("Create")
                .on_press(WorkflowMessage::ShowCreateWorkflow(true)),
        ]
        .spacing(8)
        .align_items(iced::Alignment::Center)
        .into()
    }

    fn create_filters(&self) -> Element<WorkflowMessage> {
        let categories: Vec<Option<WorkflowCategory>> = std::iter::once(None)
            .chain(self.manager.get_categories().into_iter().map(Some))
            .collect();

        let shells: Vec<Option<Shell>> = vec![
            None,
            Some(Shell::Bash),
            Some(Shell::Zsh),
            Some(Shell::Fish),
        ];

        row![
            text("Category:"),
            pick_list(
                categories,
                self.selected_category.clone(),
                WorkflowMessage::CategorySelected
            )
            .placeholder("All Categories"),
            
            text("Shell:"),
            pick_list(
                shells,
                self.selected_shell.clone(),
                WorkflowMessage::ShellSelected
            )
            .placeholder("All Shells"),
            
            text(format!("{} workflows found", self.search_results.len()))
                .style(|theme| iced::widget::text::Appearance {
                    color: Some(theme.palette().text.scale_alpha(0.7)),
                }),
        ]
        .spacing(12)
        .align_items(iced::Alignment::Center)
        .into()
    }

    fn create_workflow_list(&self) -> Element<WorkflowMessage> {
        if self.search_results.is_empty() {
            return container(
                text("No workflows found")
                    .style(|theme| iced::widget::text::Appearance {
                        color: Some(theme.palette().text.scale_alpha(0.7)),
                    })
            )
            .center_x()
            .center_y()
            .height(iced::Length::Fixed(200.0))
            .into();
        }

        scrollable(
            column(
                self.search_results
                    .iter()
                    .map(|result| self.create_workflow_card(result))
                    .collect::<Vec<_>>()
            )
            .spacing(8)
        )
        .height(iced::Length::Fixed(300.0))
        .into()
    }

    fn create_workflow_card(&self, result: &WorkflowSearchResult) -> Element<WorkflowMessage> {
        let workflow = &result.workflow;
        let is_selected = self.selected_workflow.as_ref()
            .map_or(false, |selected| selected.name == workflow.name);

        let card_content = column![
            row![
                text(&workflow.name)
                    .size(16)
                    .style(move |theme| iced::widget::text::Appearance {
                        color: Some(if is_selected {
                            theme.palette().primary
                        } else {
                            theme.palette().text
                        }),
                    }),
                // Spacer
                iced::widget::horizontal_space(iced::Length::Fill),
                text(format!("Used {} times", workflow.usage_count))
                    .size(12)
                    .style(|theme| iced::widget::text::Appearance {
                        color: Some(theme.palette().text.scale_alpha(0.6)),
                    }),
            ]
            .align_items(iced::Alignment::Center),
            
            if let Some(description) = &workflow.description {
                text(description)
                    .size(14)
                    .style(|theme| iced::widget::text::Appearance {
                        color: Some(theme.palette().text.scale_alpha(0.8)),
                    })
                    .into()
            } else {
                iced::widget::Space::new(0, 0).into()
            },
            
            if !workflow.tags.is_empty() {
                row(
                    workflow.tags
                        .iter()
                        .map(|tag| {
                            container(
                                text(tag)
                                    .size(12)
                                    .style(|theme| iced::widget::text::Appearance {
                                        color: Some(theme.palette().primary),
                                    })
                            )
                            .padding([2, 6])
                            .style(|theme| iced::widget::container::Appearance {
                                background: Some(theme.palette().primary.scale_alpha(0.1).into()),
                                border: iced::Border {
                                    color: theme.palette().primary.scale_alpha(0.3),
                                    width: 1.0,
                                    radius: 12.0.into(),
                                },
                                ..Default::default()
                            })
                            .into()
                        })
                        .collect::<Vec<_>>()
                )
                .spacing(4)
                .into()
            } else {
                iced::widget::Space::new(0, 0).into()
            },
            
            button("Select")
                .on_press(WorkflowMessage::WorkflowSelected(workflow.name.clone()))
                .style(if is_selected {
                    button::primary
                } else {
                    button::secondary
                }),
        ]
        .spacing(8);

        container(card_content)
            .padding(12)
            .style(move |theme| iced::widget::container::Appearance {
                background: Some(if is_selected {
                    theme.palette().primary.scale_alpha(0.05).into()
                } else {
                    theme.palette().background.into()
                }),
                border: iced::Border {
                    color: if is_selected {
                        theme.palette().primary
                    } else {
                        theme.palette().text.scale_alpha(0.1)
                    },
                    width: if is_selected { 2.0 } else { 1.0 },
                    radius: 8.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    fn create_workflow_details(&self) -> Element<WorkflowMessage> {
        if let Some(workflow) = &self.selected_workflow {
            column![
                text("Workflow Details").size(18),
                
                // Command preview
                container(
                    column![
                        text("Command:").size(14),
                        container(
                            text(&workflow.command)
                                .style(|theme| iced::widget::text::Appearance {
                                    color: Some(theme.palette().text.scale_alpha(0.9)),
                                })
                        )
                        .padding(8)
                        .style(|theme| iced::widget::container::Appearance {
                            background: Some(theme.palette().background.scale_alpha(0.5).into()),
                            border: iced::Border {
                                color: theme.palette().text.scale_alpha(0.2),
                                width: 1.0,
                                radius: 4.0.into(),
                            },
                            ..Default::default()
                        })
                    ]
                    .spacing(4)
                ),
                
                // Arguments
                if !workflow.arguments.is_empty() {
                    column![
                        text("Arguments:").size(14),
                        column(
                            workflow.arguments
                                .iter()
                                .map(|arg| self.create_argument_input(arg))
                                .collect::<Vec<_>>()
                        )
                        .spacing(8)
                    ]
                    .spacing(8)
                    .into()
                } else {
                    iced::widget::Space::new(0, 0).into()
                },
                
                // Actions
                row![
                    button("Execute")
                        .on_press(WorkflowMessage::ExecuteWorkflow)
                        .style(button::primary),
                    button("Dry Run")
                        .on_press(WorkflowMessage::DryRunWorkflow),
                    button("Details")
                        .on_press(WorkflowMessage::ShowWorkflowDetails(true)),
                ]
                .spacing(8),
            ]
            .spacing(12)
            .into()
        } else {
            iced::widget::Space::new(0, 0).into()
        }
    }

    fn create_argument_input(&self, arg: &WorkflowArgument) -> Element<WorkflowMessage> {
        let current_value = self.argument_values
            .get(&arg.name)
            .cloned()
            .unwrap_or_default();

        let input_element = match arg.arg_type {
            ArgumentType::Boolean => {
                let options = vec!["true".to_string(), "false".to_string()];
                pick_list(
                    options,
                    if current_value.is_empty() { None } else { Some(current_value) },
                    move |value| WorkflowMessage::ArgumentChanged(arg.name.clone(), value)
                )
                .placeholder("Select...")
                .into()
            }
            ArgumentType::Enum => {
                if let Some(options) = &arg.options {
                    pick_list(
                        options.clone(),
                        if current_value.is_empty() { None } else { Some(current_value) },
                        move |value| WorkflowMessage::ArgumentChanged(arg.name.clone(), value)
                    )
                    .placeholder("Select...")
                    .into()
                } else {
                    text_input("Value...", &current_value)
                        .on_input(move |value| WorkflowMessage::ArgumentChanged(arg.name.clone(), value))
                        .into()
                }
            }
            _ => {
                text_input("Value...", &current_value)
                    .on_input(move |value| WorkflowMessage::ArgumentChanged(arg.name.clone(), value))
                    .into()
            }
        };

        column![
            row![
                text(&arg.name)
                    .style(|theme| iced::widget::text::Appearance {
                        color: Some(theme.palette().text),
                    }),
                if arg.required {
                    text("*")
                        .style(|theme| iced::widget::text::Appearance {
                            color: Some(theme.palette().danger),
                        })
                        .into()
                } else {
                    iced::widget::Space::new(0, 0).into()
                }
            ]
            .spacing(4),
            
            input_element,
            
            if let Some(description) = &arg.description {
                text(description)
                    .size(12)
                    .style(|theme| iced::widget::text::Appearance {
                        color: Some(theme.palette().text.scale_alpha(0.7)),
                    })
                    .into()
            } else {
                iced::widget::Space::new(0, 0).into()
            },
        ]
        .spacing(8)
        .into()
    }

    fn create_workflow_dialog(&self) -> Element<WorkflowMessage> {
        // Placeholder for workflow creation dialog
        container(text("Create Workflow Dialog Placeholder"))
            .center_x()
            .center_y()
            .into()
    }

    pub fn render_editor(&self, frame: &mut Frame, area: Rect) {
        self.workflow_editor.render(frame, area);
    }
}

#[derive(Debug, Clone)]
pub struct WorkflowExecutionRequest {
    workflow: Workflow,
    arguments: HashMap<String, String>,
    dry_run: bool,
}

pub struct WorkflowEditor {
    // Add state for editing a workflow
}

impl WorkflowEditor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(Span::styled("Workflow Editor", Style::default().fg(Color::LightGreen)));

        let paragraph = Paragraph::new(Line::from("Workflow editor content goes here."))
            .block(block)
            .wrap(ratatui::widgets::Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }
}
