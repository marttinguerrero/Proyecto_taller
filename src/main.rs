use git_rustico::hash::GitHash;
use git_rustico::ignore::Ignore;
use git_rustico::index::Index;
use git_rustico::log_file::{send_text_to_log_finish, send_text_to_log_initial, LogFile};
use git_rustico::ls_file::LsFile;
use git_rustico::ls_tree::LsTree;
use git_rustico::network_commands::{self, clone_command, pull_command, push_command};
use git_rustico::rebase::Rebase;
use git_rustico::remote::Remote;
use git_rustico::repo_paths::RepoPaths;
use git_rustico::show_ref::ShowRef;
// use git_rustico::gui2;
use git_rustico::tag::Tag;
use git_rustico::{
    branch::Branch,
    commit::Commit,
    config::RepoConfig,
    git_errors::{
        command_error::CommandError::{IncorrectAmount, IncorrectOptionAmount},
        errors::ErrorType,
    },
    merge::Merge,
    refs::BranchRef,
};
use std::fs;
use std::path::Path;

fn main() {
    use std::env;
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("You must include a commmand");
        return;
    }
    let command = &args[1];
    let args = args[2..].to_vec();
    let repo_paths = match get_repo_paths() {
        Ok(paths) => paths,
        Err(e) => {
            eprintln!("{}", e);
            return;
        }
    };
    let path_log = repo_paths.get_log_file_path();
    let path_config_log = repo_paths.get_config();
    let mut repo_exist = check_repo_exists().is_ok();
    let mut text_lines = Vec::new();
    text_lines.push(send_text_to_log_initial(
        command.clone().to_string(),
        args.clone(),
        path_config_log.clone(),
    ));
    match match_command(command, args.clone(), repo_paths) {
        Ok(result) => {
            if !repo_exist {
                repo_exist = true;
            }
            text_lines.push(send_text_to_log_finish(
                command.clone().to_string(),
                result,
                path_config_log,
                true,
            ));
        }
        Err(e) => {
            let text = e.to_string();
            if repo_exist {
                text_lines.push(send_text_to_log_finish(
                    command.clone().to_string(),
                    text.clone(),
                    path_config_log,
                    false,
                ));
            }
            eprintln!("{}", text);
        }
    }
    if repo_exist {
        LogFile::write_log_file_whitout_thread(path_log, text_lines);
    }
}

fn match_command(
    command: &str,
    mut args: Vec<String>,
    repo_paths: RepoPaths,
) -> Result<String, ErrorType> {
    if command != "init" && command != "clone" {
        check_repo_exists()?;
    }
    let rep = repo_paths.clone();

    match command {
        "init" => {
            check_arguments_quantity(0, &args)?;
            let result = git_rustico::init::git_init(repo_paths)?;
            println!("{}", result);
            Ok(result)
        }

        "config" => RepoConfig::config_command(rep.clone(), args),

        "cat-file" => {
            check_arguments_quantity(2, &args)?;
            let options: Vec<String> = parse_options(&mut args);
            if options.len() > 1 {
                return Err(ErrorType::CommandError(IncorrectOptionAmount(
                    1,
                    options.len(),
                )));
            }
            let string = git_rustico::cat_file::cat_file(&options[0], &args[0])?;
            println!("{}", string);
            Ok(format!("option: {}, file: {}", options[0], args[0]))
        }
        "status" => {
            check_arguments_quantity(0, &args)?;
            Index::status_command(&repo_paths)
        }
        "check-ignore" => {
            Ignore::check_ignore_command(args, &repo_paths)?;
            Ok("".to_string()) // todo fix
        }
        "add" => {
            Index::add_command(args.clone(), &repo_paths)?;
            Ok(format!("Files added: {}.", args.join(", ")))
        }
        "rm" => {
            Index::rm_command(args.clone(), repo_paths.get_index())?;
            Ok(format!("Files removed: {}.", args.join(", ")))
        }
        "hash-object" => {
            GitHash::hash_object_command(args.clone(), repo_paths)?;
            Ok(format!("Files: {}.", args.join(", ")))
        }
        "remote" => {
            Remote::remote_command(args.clone(), repo_paths.get_remote())?;
            Ok(format!("Request: {}.", args.join(" ")))
        }
        "fetch" => {
            network_commands::fetch_command(repo_paths)?;
            Ok("".to_string())
        }
        "clone" => match clone_command(repo_paths.clone(), args) {
            Ok(_) => Ok("".to_string()),
            Err(e) => {
                fs::remove_dir_all(repo_paths.get_home().join(".git-rustico"))?;
                Err(e)
            }
        },
        "pull" => {
            pull_command(repo_paths, args)?;
            Ok("".to_string())
        }
        "push" => {
            push_command(repo_paths, args)?;
            Ok("".to_string())
        }

        "commit" => {
            Commit::commit_command(&repo_paths, args.clone())?;
            Ok(format!("Whit name: {}.", args[0].clone()))
        }

        "log" => {
            let n = BranchRef::log_command(repo_paths)?;
            Ok(format!("Number of commits: {}.", n))
        }

        "branch" => Branch::branch_command(&repo_paths, args),

        "checkout" => BranchRef::checkout_command(repo_paths, args),

        "switch" => BranchRef::switch_command(repo_paths, args),

        "merge" => {
            Merge::merge_command(repo_paths, args.clone())?;
            Ok(format!("Whit branch {}.", args[0].clone()))
        }

        "show-ref" => {
            let text = ShowRef::show_ref(args.clone())?;
            println!("{}", text);
            return_option(args)
        }

        "rebase" => Rebase::rebase(repo_paths, args),

        "ls-tree" => {
            let text = LsTree::show_tree(args.clone(), &repo_paths)?;
            println!("{}", text);
            return_option(args)
        }

        "ls-files" => {
            let text = LsFile::show_file(args.clone(), &repo_paths)?;
            println!("{}", text);
            return_option(args)
        }

        "tag" => Tag::command_tag(args, &repo_paths),

        _ => Err(ErrorType::FormatError(format!(
            "Command not implemented yet : {command}"
        ))),
    }
}

fn check_repo_exists() -> Result<(), ErrorType> {
    let path = Path::new(".git-rustico");
    match path.exists() {
        true => Ok(()),
        false => Err(ErrorType::RepositoryError(
            "Not a git-rustico repository".to_string(),
        )),
    }
}

//todo delegarlo al comando
fn check_arguments_quantity(quantity: usize, args: &[String]) -> Result<(), ErrorType> {
    if args.len() != quantity {
        return Err(ErrorType::CommandError(IncorrectAmount(
            quantity.to_string(),
            args.len(),
        )));
    }
    Ok(())
}

fn parse_options(args: &mut Vec<String>) -> Vec<String> {
    let mut filtered_strings = Vec::new();
    filtered_strings.extend(args.iter().filter(|s| s.starts_with('-')).cloned());
    args.retain(|s| !s.starts_with('-'));
    filtered_strings
}

fn return_option(args: Vec<String>) -> Result<String, ErrorType> {
    match args.is_empty() {
        true => Ok("Whithout options.".to_string()),
        false => Ok(format!("Whit options: {}.", args.join(", "))),
    }
}

fn get_repo_paths() -> Result<RepoPaths, ErrorType> {
    let current_dir = std::env::current_dir()?;
    RepoPaths::new(current_dir)
}
