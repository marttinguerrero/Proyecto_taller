use git_rustico::branch::Branch;
use git_rustico::commit::Commit;
use git_rustico::config::RepoConfig;
use git_rustico::git_errors::errors::ErrorType;
use git_rustico::gui::send_event::SendEvent;
use git_rustico::gui::ui_event::UiEvent;
use git_rustico::index::Index;
use git_rustico::init::git_init;
use git_rustico::merge::Merge;
use git_rustico::network_commands::{clone_command, pull_command, push_command};
use git_rustico::refs::BranchRef;
use git_rustico::remote::Remote;
use git_rustico::repo_paths::RepoPaths;
use glib::Priority;
use gtk::glib;
use gtk::prelude::*;
use gtk::{Builder, Button, Entry, Label, ListBox, ListBoxRow};
use gtk::{TextBuffer, TextView};
use std::cell::RefCell;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::rc::Rc;

struct BotonCommitParams {
    user: Entry,
    boton_commit: Button,
    directory: Rc<RefCell<Entry>>,
    listbox_commit: Rc<RefCell<gtk::ListBox>>,
    commit_mensaje: Entry,
    listbox_history: ListBox,
    history_scrolled_window: gtk::ScrolledWindow,
    sender: std::sync::mpsc::Sender<UiEvent>,
    branches_scrolled_window: gtk::ScrolledWindow,
    dir_entry: Rc<RefCell<Entry>>,
}

pub struct BotonesMergeParams {
    boton_guardar: Button,
    boton_conflicto: Button,
    boton_merge: Button,
}

pub fn load_ui_from_file(filename: &str) -> Builder {
    Builder::from_file(filename)
}
pub fn main() -> Result<(), ErrorType> {
    let (sender_back, receiver_back) = std::sync::mpsc::channel::<UiEvent>();
    let (sender_front, receiver_front) =
        glib::MainContext::channel::<SendEvent>(Priority::default());
    background_event(sender_front.clone(), receiver_back);
    create_window(sender_back.clone(), receiver_front);

    Ok(())
}

pub fn handle_init_event(repo_paths: RepoPaths) -> SendEvent {
    let to_be_printed = "".to_string();
    let _result = match git_init(repo_paths) {
        Ok(to_be_printed) => to_be_printed,
        Err(to_be_printed) => format!("Git init exited whit error {}", to_be_printed),
    };

    SendEvent::ToPrint(to_be_printed)
}

pub fn background_event(
    sender: glib::Sender<SendEvent>,
    receiver: std::sync::mpsc::Receiver<UiEvent>,
) {
    std::thread::spawn(move || {
        for data in receiver {
            match data {
                UiEvent::GiIinit(repo_paths) => {
                    let event = handle_init_event(repo_paths);
                    let _ = sender.send(event);
                }
                UiEvent::AddCommand(vector, repo_paths) => {
                    let event = handle_add_event(vector, repo_paths);
                    let _ = sender.send(event);
                }
                UiEvent::CommitCommand(vector, repo_paths) => {
                    let event = handle_commit_event(vector, repo_paths);
                    let _ = sender.send(event);
                }
                UiEvent::ConfigCommand(repo_paths, vector) => {
                    let event = handle_config_event(vector, repo_paths);
                    let _ = sender.send(event);
                }
                UiEvent::CheckoutCommand(repo_paths, vector) => {
                    let event = handle_checkout_event(vector, repo_paths);
                    let _ = sender.send(event);
                }
                UiEvent::BranchCommand(repo_paths, vector) => {
                    let event = handle_branch_event(vector, repo_paths);
                    let _ = sender.send(event);
                }
                UiEvent::MergeCommand(repo_paths, vector) => {
                    let event = handle_merge_event(vector, repo_paths);
                    let _ = sender.send(event);
                }
                UiEvent::RemoteCommand(vector, pathbuf) => {
                    let event = handle_remote_event(vector, pathbuf);
                    let _ = sender.send(event);
                }
                UiEvent::CloneCommand(repo_paths, vector) => {
                    let event = handle_clone_event(vector, repo_paths);
                    let _ = sender.send(event);
                }
                UiEvent::PushCommand(repo_paths, vector) => {
                    let event = handle_push_event(vector, repo_paths);
                    let _ = sender.send(event);
                }
                UiEvent::PullCommand(repo_paths, vector) => {
                    let event = handle_pull_event(repo_paths, vector);
                    let _ = sender.send(event);
                }
            }
        }
    });
}

pub fn handle_pull_event(repo_paths: RepoPaths, vector: Vec<String>) -> SendEvent {
    match pull_command(repo_paths, vector) {
        Ok(to_be_printed) => to_be_printed,
        Err(_to_be_printed) => (),
    };
    SendEvent::NotToPrint()
}
pub fn handle_push_event(vector: Vec<String>, repo_paths: RepoPaths) -> SendEvent {
    match push_command(repo_paths, vector) {
        Ok(to_be_printed) => to_be_printed,
        Err(_to_be_printed) => (),
    };
    SendEvent::NotToPrint()
}

pub fn handle_clone_event(vector: Vec<String>, repo_paths: RepoPaths) -> SendEvent {
    match clone_command(repo_paths, vector) {
        Ok(to_be_printed) => to_be_printed,
        Err(_to_be_printed) => (),
    };
    SendEvent::NotToPrint()
}

pub fn handle_remote_event(vector: Vec<String>, pathbuf: PathBuf) -> SendEvent {
    match Remote::remote_command(vector, pathbuf) {
        Ok(to_be_printed) => to_be_printed,
        Err(_to_be_printed) => (),
    };
    SendEvent::NotToPrint()
}

pub fn handle_merge_event(vector: Vec<String>, repo_paths: RepoPaths) -> SendEvent {
    match Merge::merge_command(repo_paths, vector) {
        Ok(to_be_printed) => to_be_printed,
        Err(_to_be_printed) => (),
    };
    SendEvent::NotToPrint()
}

pub fn handle_branch_event(vector: Vec<String>, repo_paths: RepoPaths) -> SendEvent {
    let _ = Branch::branch_command(&repo_paths, vector);
    SendEvent::NotToPrint()
}

pub fn handle_checkout_event(vector: Vec<String>, repo_paths: RepoPaths) -> SendEvent {
    let _ = BranchRef::checkout_command(repo_paths, vector);
    SendEvent::NotToPrint()
}

pub fn handle_config_event(vector: Vec<String>, repo_paths: RepoPaths) -> SendEvent {
    let _ = RepoConfig::config_command(repo_paths, vector);
    SendEvent::NotToPrint()
}

pub fn handle_commit_event(vector: Vec<String>, repo_paths: RepoPaths) -> SendEvent {
    match Commit::commit_command(&repo_paths, vector) {
        Ok(to_be_printed) => to_be_printed,
        Err(_to_be_printed) => (),
    };
    SendEvent::NotToPrint()
}
pub fn handle_add_event(vector: Vec<String>, repo_paths: RepoPaths) -> SendEvent {
    match Index::add_command(vector, &repo_paths) {
        Ok(to_be_printed) => to_be_printed,
        Err(_to_be_printed) => (),
    };

    SendEvent::NotToPrint()
}

pub fn create_window(
    sender: std::sync::mpsc::Sender<UiEvent>,
    _receiver: glib::Receiver<SendEvent>,
) {
    gtk::init().expect("Failed to initialize GTK.");

    // Crear una ventana principal
    let builder = load_ui_from_file("./gui/glade1.glade");

    let cloned_builder = builder.clone();

    let main_window: gtk::Window = builder
        .object("MainWindow")
        .expect("No se pudo obtener la ventana");

    let local_scrolled_window: gtk::ScrolledWindow = builder
        .object("LocalScrolledWindow")
        .expect("No se pudo pa");

    let history_scrolled_window: gtk::ScrolledWindow = builder
        .object("HistoryScrolledWindow")
        .expect("No se pudo pa");

    let add_scrolled_window: gtk::ScrolledWindow =
        builder.object("AddScrolledWindow").expect("No se pudo pa");

    let commit_scrolled_window: gtk::ScrolledWindow = builder
        .object("CommitScrolledWindow")
        .expect("No se puedo pa");

    let merge_scrolled_window: gtk::ScrolledWindow = builder
        .object("MergeScrolledWindow")
        .expect("No se pudo pa");

    let user: Entry = match cloned_builder.object("UserEntry") {
        Some(entry) => entry,
        None => {
            println!("Error: ");
            Entry::new()
        }
    };

    let agregar_branch_entry: Entry = match cloned_builder.object("AddBranchEntry") {
        Some(entry) => entry,
        None => {
            println!("Error: ");
            Entry::new()
        }
    };

    let branches_scrolled_window: gtk::ScrolledWindow = builder
        .object("BranchesScrolledWindow")
        .expect("No se pudo pa");

    let commit_mensaje: Entry = match cloned_builder.object("CommitEntry") {
        Some(entry) => entry,
        None => {
            println!("Error: ");
            Entry::new()
        }
    };

    let mail: Entry = match cloned_builder.object("MailEntry") {
        Some(entry) => entry,
        None => {
            println!("Error: ");
            Entry::new()
        }
    };
    let archivo_agregar: Entry = match cloned_builder.object("add_entry") {
        Some(entry) => entry,
        None => {
            println!("Error: ");
            Entry::new()
        }
    };

    let repo_a_remote: Entry = match cloned_builder.object("RemoteEntry") {
        Some(entry) => entry,
        None => {
            println!("Error: ");
            Entry::new()
        }
    };

    let repo_name: Entry = match cloned_builder.object("RepoName") {
        Some(entry) => entry,
        None => {
            println!("Error: ");
            Entry::new()
        }
    };

    let change_branch_entry: Rc<RefCell<Entry>> = Rc::new(RefCell::new(
        match cloned_builder.object("ChangeBranchEntry") {
            Some(entry) => entry,
            None => {
                println!("Error: ");
                Entry::new()
            }
        },
    ));
    let _merge_rama: Entry = match cloned_builder.object("MergeEntry") {
        Some(entry) => entry,
        None => {
            println!("Error: ");
            Entry::new()
        }
    };

    let clone_entry: Entry = match cloned_builder.object("CloneEntry") {
        Some(entry) => entry,
        None => {
            println!("Error: ");
            Entry::new()
        }
    };

    let pull_entry: Entry = match cloned_builder.object("PullEntry") {
        Some(entry) => entry,
        None => {
            println!("Error: ");
            Entry::new()
        }
    };

    let merge_entry: Rc<RefCell<Entry>> =
        Rc::new(RefCell::new(match cloned_builder.object("MergeEntry") {
            Some(entry) => entry,
            None => {
                println!("Error: ");
                Entry::new()
            }
        }));

    let listbox_branch: ListBox = ListBox::new();
    let listbox_local: Rc<RefCell<gtk::ListBox>> = Rc::new(RefCell::new(ListBox::new()));
    let listbox_add: Rc<RefCell<gtk::ListBox>> = Rc::new(RefCell::new(ListBox::new()));
    let _listbox_remove: ListBox = ListBox::new();
    let listbox_commit: Rc<RefCell<gtk::ListBox>> = Rc::new(RefCell::new(ListBox::new()));
    let listbox_history: ListBox = ListBox::new();
    let boton_init: Button = builder.object("InitBoton").expect("Ashe");
    let boton_agregar: Button = builder.object("boton_agregar").expect("no se pudo pa");
    let boton_commit: Button = builder.object("CommitButton").expect("no se pudo pa");
    let boton_add_branch: Button = builder.object("AddBranchButton").expect("no se pudo pa");
    let boton_actulizar: Button = builder.object("ActualizarButton").expect("no se pudo pa");
    let boton_push: Button = builder.object("PushBoton").expect("msg");
    let boton_clone: Button = builder.object("CloneBoton").expect("msg");
    let boton_pull: Button = builder.object("PullBoton").expect("msg");
    let boton_remote: Button = builder.object("RemoteBoton").expect("msg");
    let boton_merge: Button = builder.object("MergeBoton").expect("msg");
    let boton_conflicto: Button = builder.object("ConflictButton").expect("msg");
    let boton_guardar: Button = builder.object("boton_guardar_cambios").expect("msg");

    let boton_change_branch: Button = builder.object("ChangeBranchButton").expect("no se pudo pa");
    let confirmar_boton: Button = builder.object("ConfigButton").expect("no se pudo pa");

    let archivo_cambiado: Rc<RefCell<Vec<PathBuf>>> = Rc::new(RefCell::new(Vec::new()));
    let directory: Rc<RefCell<Entry>> =
        Rc::new(RefCell::new(match cloned_builder.object("init_entry") {
            Some(entry) => entry,
            None => {
                println!("Error: ");
                Entry::new()
            }
        }));

    let dir_entry: Rc<RefCell<Entry>> =
        Rc::new(RefCell::new(match cloned_builder.object("DirEntry") {
            Some(entry) => entry,
            None => {
                println!("Error: ");
                Entry::new()
            }
        }));

    let local_scrolled_window_clone = local_scrolled_window.clone();
    let add_scrolled_window_clone = add_scrolled_window.clone();
    let row_branch = ListBoxRow::new();
    let label_branch = Label::new(Some("master"));
    row_branch.add(&label_branch);
    listbox_branch.add(&row_branch);
    branches_scrolled_window.add(&listbox_branch);
    branches_scrolled_window.show_all();

    handle_init_button(
        boton_init,
        listbox_local.clone(),
        listbox_add.clone(),
        directory.clone(),
        add_scrolled_window_clone.clone(),
        local_scrolled_window_clone.clone(),
        sender.clone(),
    );
    handle_boton_agregar(
        boton_agregar,
        directory.clone(),
        archivo_agregar,
        &mut listbox_commit.clone(),
        commit_scrolled_window,
        sender.clone(),
        dir_entry.clone(),
    );

    handle_config_button(
        confirmar_boton,
        directory.clone(),
        user.clone(),
        mail,
        sender.clone(),
        dir_entry.clone(),
    );

    handle_branch_button(
        boton_add_branch,
        agregar_branch_entry,
        listbox_branch,
        branches_scrolled_window.clone(),
        directory.clone(),
        sender.clone(),
        dir_entry.clone(),
    );

    handle_checkout_button(
        boton_change_branch,
        change_branch_entry.borrow_mut().deref().clone(),
        directory.clone(),
        sender.clone(),
        dir_entry.clone(),
    );
    handle_boton_remote(
        boton_remote,
        repo_name,
        repo_a_remote,
        directory.clone(),
        sender.clone(),
        dir_entry.clone(),
    );

    handle_boton_clone(boton_clone, clone_entry, sender.clone(), dir_entry.clone());

    handle_boton_push(
        boton_push,
        directory.clone(),
        sender.clone(),
        dir_entry.clone(),
    );
    handle_boton_pull(
        boton_pull,
        directory.clone(),
        pull_entry,
        sender.clone(),
        dir_entry.clone(),
    );

    main_window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });
    boton_actulizar.connect_clicked({
        let listbox_local = listbox_local.clone();
        let listbox_add = listbox_add.clone();

        let mut directory = directory.clone();
        if directory.borrow_mut().deref().text() == "" {
            directory = dir_entry.clone()
        }
        move |_| {
            if let Err(e) = handle_boton_actualizar(
                listbox_add.clone(),
                listbox_local.clone(),
                directory.clone(),
                add_scrolled_window_clone.clone(),
                local_scrolled_window_clone.clone(),
            ) {
                match e {
                    ErrorType::RepositoryError(_e) => todo!(),
                    _ => eprintln!("{}", e),
                }
            }
        }
    });
    let params_merge = BotonesMergeParams {
        boton_guardar,
        boton_conflicto,
        boton_merge,
    };
    handle_merge_button(
        params_merge,
        dir_entry.clone(),
        merge_entry,
        directory.clone(),
        merge_scrolled_window,
        sender.clone(),
        archivo_cambiado,
    );
    let params = BotonCommitParams {
        user,
        boton_commit,
        directory,
        listbox_commit,
        commit_mensaje,
        listbox_history,
        history_scrolled_window,
        sender,
        branches_scrolled_window,
        dir_entry,
    };
    handle_boton_commit(params);

    main_window.show_all();

    gtk::main();
}

pub fn handle_boton_pull(
    boton_pull: Button,
    directory: Rc<RefCell<Entry>>,
    pull_entry: Entry,
    sender: std::sync::mpsc::Sender<UiEvent>,
    dir_entry: Rc<RefCell<Entry>>,
) {
    boton_pull.connect_clicked(move |_| {
        if directory.borrow().deref().text() != "" {
            let repo_paths = RepoPaths::new(PathBuf::from(
                directory.borrow_mut().deref().text().to_string(),
            ))
            .unwrap();
            let vector = vec![pull_entry.text().to_string()];
            let _ = sender.send(UiEvent::PullCommand(repo_paths, vector));
        } else {
            let repo_paths = RepoPaths::new(PathBuf::from(
                dir_entry.borrow_mut().deref().text().to_string(),
            ))
            .unwrap();
            let vector = vec![pull_entry.text().to_string()];
            let _ = sender.send(UiEvent::PullCommand(repo_paths, vector));
        }
    });
}

pub fn handle_boton_push(
    boton_push: Button,
    directory: Rc<RefCell<Entry>>,
    sender: std::sync::mpsc::Sender<UiEvent>,
    dir_entry: Rc<RefCell<Entry>>,
) {
    boton_push.connect_clicked(move |_| {
        if directory.borrow().deref().text() != "" {
            let repo_paths = RepoPaths::new(PathBuf::from(
                directory.borrow_mut().deref().text().to_string(),
            ))
            .unwrap();
            let vector = Vec::new();
            let _ = sender.send(UiEvent::PushCommand(repo_paths, vector));
        } else {
            let repo_paths = RepoPaths::new(PathBuf::from(
                dir_entry.borrow_mut().deref().text().to_string(),
            ))
            .unwrap();
            let vector = Vec::new();
            let _ = sender.send(UiEvent::PushCommand(repo_paths, vector));
        }
    });
}

pub fn handle_boton_clone(
    boton_clone: Button,
    clone_entry: Entry,
    sender: std::sync::mpsc::Sender<UiEvent>,
    dir_entry: Rc<RefCell<Entry>>,
) {
    boton_clone.connect_clicked(move |_| {
        let repo_paths = RepoPaths::new(PathBuf::from(
            dir_entry.borrow_mut().deref().text().to_string(),
        ))
        .unwrap();
        let vector = vec![clone_entry.text().to_string()];
        let _ = sender.send(UiEvent::CloneCommand(repo_paths, vector));
    });
}

pub fn handle_boton_remote(
    boton_remote: Button,
    remote_name: Entry,
    repo_a_remote: Entry,
    directory: Rc<RefCell<Entry>>,
    sender: std::sync::mpsc::Sender<UiEvent>,
    dir_entry: Rc<RefCell<Entry>>,
) {
    boton_remote.connect_clicked(move |_| {
        if directory.borrow_mut().deref().text() != "" {
            let repo_paths = RepoPaths::new(PathBuf::from(
                directory.borrow_mut().deref().text().to_string(),
            ))
            .unwrap();
            let vector = vec![
                "add".to_string(),
                remote_name.text().to_string(),
                repo_a_remote.text().to_string(),
            ];
            let _ = sender.send(UiEvent::RemoteCommand(vector, repo_paths.get_remote()));
        } else {
            let repo_paths = RepoPaths::new(PathBuf::from(
                dir_entry.borrow_mut().deref().text().to_string(),
            ))
            .unwrap();
            let url = "git://127.0.0.1/".to_string();
            let url_final = format!("{}{}", url, repo_a_remote.text());
            let vector = vec!["add".to_string(), remote_name.text().to_string(), url_final];
            let _ = sender.send(UiEvent::RemoteCommand(vector, repo_paths.get_remote()));
        }
    });
}

fn write_to_file(file_path: &PathBuf, content: &str) -> std::io::Result<()> {
    let mut file = File::create(file_path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

pub fn handle_merge_button(
    params: BotonesMergeParams,
    dir_entry: Rc<RefCell<Entry>>,
    cambiar_branch_entry: Rc<RefCell<Entry>>,
    directory: Rc<RefCell<Entry>>,
    merge_scrolled_window: gtk::ScrolledWindow,
    sender: std::sync::mpsc::Sender<UiEvent>,
    archivo_cambiado: Rc<RefCell<Vec<PathBuf>>>,
) {
    let directory_clone = Rc::clone(&directory);
    let dir_entry_clone = Rc::clone(&dir_entry);
    let cambiar_branch_entry_clone = Rc::clone(&cambiar_branch_entry);
    let archivo_cambiado_clone = Rc::clone(&archivo_cambiado);

    // Crear el TextView y el TextBuffer
    let text_view = Rc::new(RefCell::new(TextView::new()));
    let text_buffer = TextBuffer::new(None::<&gtk::TextTagTable>);
    let text_buffer_clone = text_buffer.clone();
    text_view.borrow().set_buffer(Some(&text_buffer));

    // Configurar el evento para el botón de guardar
    let text_view_clone = Rc::clone(&text_view);
    params.boton_guardar.connect_clicked(move |_| {
        let archivo_cambiados = archivo_cambiado.borrow();
        let nombre_archivo = archivo_cambiados.first().unwrap();
        // Acceder al TextView desde el cierre del botón de guardar
        let _buffer = text_view_clone
            .borrow()
            .buffer()
            .expect("Failed to get buffer.");
        let start = text_buffer_clone.clone().start_iter();
        let end = text_buffer_clone.clone().end_iter();
        write_to_file(
            nombre_archivo,
            text_buffer_clone.text(&start, &end, true).as_ref().unwrap(),
        )
        .unwrap();

        // Puedes hacer más cosas con el contenido del TextView aquí
    });

    // Configurar el evento para el botón de conflicto
    params.boton_conflicto.connect_clicked(move |_| {
        if directory_clone.borrow_mut().deref().text() != "" {
            let mut archivo_cambiado = archivo_cambiado_clone.borrow_mut();
            let repo_paths = RepoPaths::new(PathBuf::from(
                directory_clone.borrow_mut().deref().text().to_string(),
            ))
            .unwrap();

            let archivos = fs::read_dir(repo_paths.get_home().clone()).unwrap();
            for archivo in archivos {
                let ruta = archivo.unwrap().path();

                if ruta.is_file() && contiene_simbolos(&ruta).unwrap() {
                    archivo_cambiado.push(ruta.clone());

                    // Actualizar el contenido del TextView
                    let file_content = read_file(ruta.to_str().unwrap());
                    let combined_content =
                        format!("El archivo {} tiene conflictos", ruta.to_string_lossy());
                    text_buffer.set_text(&combined_content);
                    text_buffer.set_text(&file_content);
                    merge_scrolled_window.add(<TextView as AsRef<gtk::Widget>>::as_ref(
                        &text_view.borrow(),
                    ));

                    merge_scrolled_window.show_all();
                }
            }
        } else {
            let mut archivo_cambiado = archivo_cambiado_clone.borrow_mut();
            let repo_paths = RepoPaths::new(PathBuf::from(
                dir_entry.borrow_mut().deref().text().to_string(),
            ))
            .unwrap();

            let archivos = fs::read_dir(repo_paths.get_home().clone()).unwrap();
            for archivo in archivos {
                let ruta = archivo.unwrap().path();

                if ruta.is_file() && contiene_simbolos(&ruta).unwrap() {
                    archivo_cambiado.push(ruta.clone());

                    // Actualizar el contenido del TextView
                    let file_content = read_file(ruta.to_str().unwrap());
                    let combined_content =
                        format!("El archivo {} tiene conflictos", ruta.to_string_lossy());
                    text_buffer.set_text(&combined_content);
                    text_buffer.set_text(&file_content);
                    merge_scrolled_window.add(<TextView as AsRef<gtk::Widget>>::as_ref(
                        &text_view.borrow(),
                    ));

                    merge_scrolled_window.show_all();
                }
            }
        }
    });

    // Configurar el evento para el botón de merge
    params.boton_merge.connect_clicked(move |_| {
        if directory.borrow_mut().deref().text() != "" {
            let repo_paths = RepoPaths::new(PathBuf::from(
                directory.borrow_mut().deref().text().to_string(),
            ))
            .unwrap();
            let vector = vec![cambiar_branch_entry_clone
                .borrow_mut()
                .deref()
                .text()
                .to_string()];
            let _ = sender.send(UiEvent::MergeCommand(repo_paths.clone(), vector));
        } else {
            let repo_paths = RepoPaths::new(PathBuf::from(
                dir_entry_clone.borrow_mut().deref().text().to_string(),
            ))
            .unwrap();
            let vector = vec![cambiar_branch_entry_clone
                .borrow_mut()
                .deref()
                .text()
                .to_string()];
            let _ = sender.send(UiEvent::MergeCommand(repo_paths.clone(), vector));
        }
    });
}

pub fn handle_checkout_button(
    boton_checkout: Button,
    cambiar_branch_entry: Entry,
    directory: Rc<RefCell<Entry>>,
    sender: std::sync::mpsc::Sender<UiEvent>,
    dir_entry: Rc<RefCell<Entry>>,
) {
    boton_checkout.connect_clicked(move |_| {
        if directory.borrow_mut().deref().text() != "" {
            let repo_paths = RepoPaths::new(PathBuf::from(
                directory.borrow_mut().deref().text().to_string(),
            ))
            .unwrap();
            let vector = vec![cambiar_branch_entry.text().to_string()];

            let _ = sender.send(UiEvent::CheckoutCommand(repo_paths, vector));
        } else {
            let repo_paths = RepoPaths::new(PathBuf::from(
                dir_entry.borrow_mut().deref().text().to_string(),
            ))
            .unwrap();
            let vector = vec![cambiar_branch_entry.text().to_string()];

            let _ = sender.send(UiEvent::CheckoutCommand(repo_paths, vector));
        }
    });
}

pub fn handle_branch_button(
    boton_agregar_branch: Button,
    agregar_branch_entry: Entry,
    listbox_branch: ListBox,
    branches_scrolled_window: gtk::ScrolledWindow,
    directory: Rc<RefCell<Entry>>,
    sender: std::sync::mpsc::Sender<UiEvent>,
    dir_entry: Rc<RefCell<Entry>>,
) {
    boton_agregar_branch.connect_clicked(move |_| {
        if directory.borrow_mut().deref().text() != "" {
            let repo_paths = RepoPaths::new(PathBuf::from(
                directory.borrow_mut().deref().text().to_string(),
            ))
            .unwrap();
            let row_branch = ListBoxRow::new();
            let label_branch = Label::new(Some(agregar_branch_entry.text().as_ref()));
            row_branch.add(&label_branch);
            listbox_branch.add(&row_branch);
            branches_scrolled_window.add(&listbox_branch);
            branches_scrolled_window.show_all();
            let vector = vec![agregar_branch_entry.text().to_string()];

            let _ = sender.send(UiEvent::BranchCommand(repo_paths, vector));
        } else {
            let repo_paths = RepoPaths::new(PathBuf::from(
                dir_entry.borrow_mut().deref().text().to_string(),
            ))
            .unwrap();
            let row_branch = ListBoxRow::new();
            let label_branch = Label::new(Some(agregar_branch_entry.text().as_ref()));
            row_branch.add(&label_branch);
            listbox_branch.add(&row_branch);
            branches_scrolled_window.add(&listbox_branch);
            branches_scrolled_window.show_all();
            let vector = vec![agregar_branch_entry.text().to_string()];

            let _ = sender.send(UiEvent::BranchCommand(repo_paths, vector));
        }
    });
}

pub fn handle_config_button(
    boton_config: Button,
    directory: Rc<RefCell<Entry>>,
    user: Entry,
    mail: Entry,
    sender: std::sync::mpsc::Sender<UiEvent>,
    dir_entry: Rc<RefCell<Entry>>,
) {
    boton_config.connect_clicked(move |_| {
        if directory.borrow_mut().deref().text() != "" {
            let repo_paths = RepoPaths::new(PathBuf::from(
                directory.borrow_mut().deref().text().to_string(),
            ))
            .unwrap();
            let vector = vec![
                "--user-name".to_string(),
                user.text().to_string(),
                "--user-mail".to_string(),
                mail.text().to_string(),
            ];

            let _ = sender.send(UiEvent::ConfigCommand(repo_paths, vector));
        } else {
            let repo_paths = RepoPaths::new(PathBuf::from(
                dir_entry.borrow_mut().deref().text().to_string(),
            ))
            .unwrap();

            let vector = vec![
                "--user-name".to_string(),
                user.text().to_string(),
                "--user-mail".to_string(),
                mail.text().to_string(),
            ];

            let _ = sender.send(UiEvent::ConfigCommand(repo_paths, vector));
        }
    });
}

pub fn handle_init_button(
    boton_init: Button,
    listbox_local: Rc<RefCell<gtk::ListBox>>,
    listbox_add: Rc<RefCell<gtk::ListBox>>,
    directory: Rc<RefCell<Entry>>,
    add_scrolled_window_clone: gtk::ScrolledWindow,
    local_scrolled_window_clone: gtk::ScrolledWindow,
    sender: std::sync::mpsc::Sender<UiEvent>,
) {
    boton_init.connect_clicked(move |_| {
        let listbox_local = listbox_local.borrow_mut();
        let listbox_local = listbox_local.deref();
        let listbox_add = listbox_add.borrow_mut();
        let listbox_add = listbox_add.deref();
        let repo_paths = RepoPaths::new(PathBuf::from(
            directory.borrow_mut().deref().text().to_string(),
        ))
        .unwrap();
        let _ = sender.send(UiEvent::GiIinit(repo_paths));
        let file_names = listar_contenido_carpeta(&directory.borrow_mut().text()).unwrap();

        for file_name in file_names {
            let row_local = ListBoxRow::new();
            let row_add = ListBoxRow::new();
            let row_remove = ListBoxRow::new();

            let label_remove = Label::new(Some(&file_name));
            let label_local = Label::new(Some(&file_name));
            let label_add = Label::new(Some(&file_name));

            row_local.add(&label_local);
            listbox_local.add(&row_local);

            row_add.add(&label_add);
            listbox_add.add(&row_add);

            row_remove.add(&label_remove);
        }
        add_scrolled_window_clone.add(listbox_add);
        local_scrolled_window_clone.add(listbox_local);
        add_scrolled_window_clone.show_all();
        local_scrolled_window_clone.show_all();
    });
}

fn handle_boton_commit(params: BotonCommitParams) {
    let listbox_commit_clones = params.listbox_commit.clone();

    params.boton_commit.connect_clicked(move |_| {
        let binding = listbox_commit_clones.borrow_mut();
        let listbox_commit = binding.deref();
        vaciar_listbox(listbox_commit);
        let message = params.commit_mensaje.text().to_string();
        let vector = vec![message.clone()];
        if params.directory.borrow_mut().deref().text() != ""{

            let repo_paths = RepoPaths::new(PathBuf::from(
                params.directory
                    .clone()
                    .borrow_mut()
                    .deref_mut()
                    .text()
                    .to_string(),
            ))
            .unwrap();
            let _ = params.sender.send(UiEvent::CommitCommand(vector, repo_paths));
            let history =
                params.user.text().to_string() + "                                                                                                " + &message;
            let row_history = ListBoxRow::new();
            let label_history = Label::new(Some(&history));
            row_history.add(&label_history);
            params.listbox_history.add(&row_history);
            params.history_scrolled_window.add(&params.listbox_history);
            params.history_scrolled_window.show_all();
            params.branches_scrolled_window.show_all();
        }
        else{
            let repo_paths = RepoPaths::new(PathBuf::from(
                params.dir_entry
                    .clone()
                    .borrow_mut()
                    .deref_mut()
                    .text()
                    .to_string(),
            ))
            .unwrap();
            let _ = params.sender.send(UiEvent::CommitCommand(vector, repo_paths));
            let history =
                params.user.text().to_string() + "                                                                                                " + &message;
            let row_history = ListBoxRow::new();
            let label_history = Label::new(Some(&history));
            row_history.add(&label_history);
            params.listbox_history.add(&row_history);
            params.history_scrolled_window.add(&params.listbox_history);
            params.history_scrolled_window.show_all();
            params.branches_scrolled_window.show_all();
        }
    });
}

fn handle_boton_agregar(
    boton_agregar: Button,
    directory: Rc<RefCell<Entry>>,
    archivo_agregar: Entry,
    listbox_commit: &mut Rc<RefCell<gtk::ListBox>>,
    commit_scrolled_window: gtk::ScrolledWindow,
    sender: std::sync::mpsc::Sender<UiEvent>,
    dir_entry: Rc<RefCell<Entry>>,
) {
    let listbox_commit_clone = listbox_commit.clone();
    boton_agregar.connect_clicked(move |_| {
        let binding = listbox_commit_clone.borrow_mut();
        let listbox_commit = binding.deref();
        if directory.borrow_mut().deref().text() != "" {
            let repo_paths = RepoPaths::new(PathBuf::from(
                directory
                    .clone()
                    .borrow_mut()
                    .deref_mut()
                    .text()
                    .to_string(),
            ))
            .unwrap();

            let vector_string = vec![archivo_agregar.text().to_string()];

            let _ = sender.send(UiEvent::AddCommand(vector_string, repo_paths));

            let row_commit = ListBoxRow::new();

            let label_commit = Label::new(Some(archivo_agregar.text().as_ref()));
            row_commit.add(&label_commit);
            listbox_commit.add(&row_commit);

            commit_scrolled_window.add(listbox_commit);
            commit_scrolled_window.show_all();
        } else {
            let repo_paths = RepoPaths::new(PathBuf::from(
                dir_entry
                    .clone()
                    .borrow_mut()
                    .deref_mut()
                    .text()
                    .to_string(),
            ))
            .unwrap();

            let vector_string = vec![archivo_agregar.text().to_string()];

            let _ = sender.send(UiEvent::AddCommand(vector_string, repo_paths));

            let row_commit = ListBoxRow::new();

            let label_commit = Label::new(Some(archivo_agregar.text().as_ref()));
            row_commit.add(&label_commit);
            listbox_commit.add(&row_commit);

            commit_scrolled_window.add(listbox_commit);
            commit_scrolled_window.show_all();
        }
    });
}

fn listar_contenido_carpeta(ruta: &str) -> Result<Vec<String>, ErrorType> {
    let mut lista_contenido = Vec::new();

    for entrada in fs::read_dir(ruta)? {
        let entrada = entrada?;
        let nombre = entrada.file_name().into_string().unwrap();
        lista_contenido.push(nombre);
    }

    Ok(lista_contenido)
}

fn vaciar_listbox(listbox: &gtk::ListBox) {
    // Iterar sobre todas las filas del ListBox y eliminarlas
    while let Some(row) = listbox.row_at_index(0) {
        listbox.remove(&row);
    }
}

fn handle_boton_actualizar(
    listbox_add: Rc<RefCell<gtk::ListBox>>,
    listbox_local: Rc<RefCell<gtk::ListBox>>,
    directory: Rc<RefCell<Entry>>,
    add_scrolled_window_clone: gtk::ScrolledWindow,
    local_scrolled_window_clone: gtk::ScrolledWindow,
) -> Result<(), ErrorType> {
    let listbox_add = listbox_add.borrow_mut();
    let listbox_add = listbox_add.deref();
    let listbox_local = listbox_local.borrow_mut();
    let listbox_local = listbox_local.deref();
    vaciar_listbox(listbox_local);
    vaciar_listbox(listbox_add);

    let file_names = listar_contenido_carpeta(&directory.borrow_mut().text())?;

    for file_name in file_names {
        let row_local = ListBoxRow::new();
        let row_add = ListBoxRow::new();

        let label_local = Label::new(Some(&file_name));
        let label_add = Label::new(Some(&file_name));

        row_local.add(&label_local);
        listbox_local.add(&row_local);

        row_add.add(&label_add);
        listbox_add.add(&row_add);
    }

    add_scrolled_window_clone.add(listbox_add);
    local_scrolled_window_clone.add(listbox_local);
    add_scrolled_window_clone.show_all();
    local_scrolled_window_clone.show_all();
    Ok(())
}

fn contiene_simbolos(ruta: &Path) -> io::Result<bool> {
    // Abre el archivo
    let archivo = fs::File::open(ruta)?;
    let lector = BufReader::new(archivo);

    // Itera sobre las líneas del archivo
    for linea in lector.lines().map_while(Result::ok) {
        // Verifica si la línea contiene "<<<<"
        if linea.contains("<<<<") {
            return Ok(true);
        }
    }

    Ok(false)
}

fn read_file(file_path: &str) -> String {
    use std::io::Read;

    let mut file = File::open(file_path).expect("Error al abrir el archivo");
    let mut content = String::new();
    file.read_to_string(&mut content)
        .expect("Error al leer el archivo");
    content
}
