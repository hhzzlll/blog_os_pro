use super::{BUFFER_WIDTH, EXECUTOR, Priority};
use crate::{io::vga_buffer::WRITER, mm::allocator::ALLOCATOR, print, println, task};
use alloc::{string::String, vec::Vec};
use core::{
    alloc::{GlobalAlloc, Layout},
    fmt::Write,
};
use pc_keyboard::{DecodedKey, KeyCode};

/// Shell 结构体
pub struct Shell {
    input_buffer: String,         // 输入缓冲区
    prompt: &'static str,         // 提示符
    cursor_position: usize,       // 光标位置
    history: Vec<String>,         // 命令历史
    history_index: Option<usize>, // 历史索引
}

impl Shell {
    /// 创建一个新的 shell
    pub fn new() -> Self {
        Shell {
            input_buffer: String::new(),
            prompt: "blog_os> ",
            cursor_position: 0,
            history: Vec::new(),
            history_index: None,
        }
    }

    /// 处理按键
    pub fn handle_key(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::RawKey(key) => match key {
                KeyCode::ArrowUp => {
                    if !self.history.is_empty() {
                        let new_index = match self.history_index {
                            None => Some(self.history.len() - 1),
                            Some(i) if i > 0 => Some(i - 1),
                            Some(_) => Some(0),
                        };
                        self.history_index = new_index;
                        if let Some(index) = new_index {
                            self.input_buffer = self.history[index].clone();
                            self.cursor_position = self.input_buffer.len();
                            self.redraw_line();
                        }
                    }
                }
                KeyCode::ArrowDown => {
                    if let Some(current_index) = self.history_index {
                        let new_index = if current_index + 1 < self.history.len() {
                            Some(current_index + 1)
                        } else {
                            None
                        };
                        self.history_index = new_index;
                        if let Some(index) = new_index {
                            self.input_buffer = self.history[index].clone();
                        } else {
                            self.input_buffer.clear();
                        }
                        self.cursor_position = self.input_buffer.len();
                        self.redraw_line();
                    }
                }
                KeyCode::ArrowLeft => {
                    if self.cursor_position > 0 {
                        self.cursor_position -= 1;
                        self.redraw_line();
                    }
                }
                KeyCode::ArrowRight => {
                    if self.cursor_position < self.input_buffer.len() {
                        self.cursor_position += 1;
                        self.redraw_line();
                    }
                }
                _ => {}
            },
            DecodedKey::Unicode(c) => match c {
                '\n' => {
                    if !self.input_buffer.trim().is_empty() {
                        self.history.push(self.input_buffer.clone());
                    }
                    self.history_index = None;
                    self.execute_command();
                    self.input_buffer.clear();
                    self.cursor_position = 0;
                    print!("{}", self.prompt);
                }
                '\x08' => {
                    // 处理退格键
                    if self.cursor_position > 0 {
                        self.input_buffer.remove(self.cursor_position - 1);
                        self.cursor_position -= 1;
                        self.redraw_line();
                    }
                }
                '\x7F' => {
                    // 处理删除键
                    if self.cursor_position < self.input_buffer.len() {
                        self.input_buffer.remove(self.cursor_position);
                        self.redraw_line();
                    }
                }
                _ => {
                    self.input_buffer.insert(self.cursor_position, c);
                    self.cursor_position += 1;
                    self.redraw_line();
                }
            },
        }
    }

    /// 执行输入缓冲区中的命令
    fn execute_command(&self) {
        println!();
        let args: Vec<&str> = self.input_buffer.split_whitespace().collect();
        if args.is_empty() {
            return;
        }

        match args[0] {
            "help" => self.cmd_help(),
            "echo" => self.cmd_echo(&args[1..]),
            "clear" => self.cmd_clear(),
            "add" => self.cmd_add(&args[1..]),
            "run" => self.cmd_run(),
            "mem" => self.cmd_mem(&args[1..]),
            "ls" => self.cmd_ls(&args[1..]),
            "cat" => self.cmd_cat(&args[1..]),
            _ => println!("Unknown command: {}", args[0]),
        }
    }

    /// 打印帮助信息
    fn cmd_help(&self) {
        println!("------------------------------------");
        println!("| Available commands:");
        println!("| - help: Print this help message");
        println!("| - echo [string]: Print the string to the screen");
        println!("| - clear: Clear the screen");
        println!("| - add <task name> <priority>: Add a new task to the task queue");
        println!("| - run: Run the task queue");
        println!("| - mem <operation> [args...]: Perform a memory operation");
        println!("| - ls [path]: List files in the given path");
        println!("| - cat <file>: Print the content of the file");
        println!("------------------------------------");
    }

    /// 打印给定的参数
    fn cmd_echo(&self, args: &[&str]) {
        if args.is_empty() {
            println!("Usage: echo <string>");
            return;
        }

        println!("{}", args.join(" "));
    }

    /// 清屏
    fn cmd_clear(&self) {
        for _ in 0..super::BUFFER_HEIGHT {
            println!();
        }
    }

    /// 添加任务到任务队列
    fn cmd_add(&self, args: &[&str]) {
        if args.is_empty() {
            println!("Usage: add <task_name> <priority>");
            return;
        }

        if args.len() < 2 {
            println!("No priority provided");
            return;
        }

        let task = match args.get(0) {
            Some(&"limit") => task::user_task::limited_time_task(10),
            _ => {
                println!("No task name provided or unknown task");
                return;
            }
        };

        let priority = match args.get(1) {
            Some(&"high") => Priority::High,
            Some(&"normal") => Priority::Normal,
            Some(&"low") => Priority::Low,
            _ => {
                println!("Invalid priority: {}", args[1]);
                return;
            }
        };

        let mut executor = EXECUTOR.lock();
        executor.spawn(task::Task::new(task, priority));
    }

    /// 运行任务队列
    fn cmd_run(&self) {
        let mut executor = EXECUTOR.lock();
        executor.run();
    }

    /// 运行内存管理
    fn cmd_mem(&self, args: &[&str]) {
        if args.is_empty() {
            println!("Usage: mem <operation> [args...]");
            println!("Operations:");
            println!("  alloc <size> - Allocate memory");
            println!("  dealloc <ptr> - Deallocate memory");
            println!("  status - Show memory status");
            return;
        }

        match args[0] {
            "alloc" => {
                if args.len() != 2 {
                    println!("Usage: mem alloc <size>");
                    return;
                }

                if let Ok(size) = args[1].parse::<usize>() {
                    unsafe {
                        let layout = Layout::from_size_align(size, 8).unwrap();
                        println!("Allocating {} bytes...", size);
                        let ptr = ALLOCATOR.alloc(layout);
                        if !ptr.is_null() {
                            println!("Allocated {} bytes at: {:p}", size, ptr);
                        } else {
                            println!("Failed to allocate {} bytes", size);
                        }
                    }
                } else {
                    println!("Invalid size: {}", args[1]);
                }
            }
            "dealloc" => {
                if args.len() != 3 {
                    println!("Usage: mem dealloc <ptr> <size>");
                    return;
                }

                if let Ok(ptr_val) = usize::from_str_radix(args[1].trim_start_matches("0x"), 16) {
                    unsafe {
                        let ptr = ptr_val as *mut u8;
                        if let Ok(size) = args[2].parse::<usize>() {
                            let layout = Layout::from_size_align(size, 8).unwrap();
                            ALLOCATOR.dealloc(ptr, layout);
                            println!("Deallocated memory at: {:p}", ptr);
                        } else {
                            println!("Invalid size: {}", args[2]);
                        }
                    }
                } else {
                    println!("Invalid pointer: {}", args[1]);
                }
            }
            "status" => unsafe {
                ALLOCATOR.lock().print_free_regions();
            },
            _ => println!("Unknown operation: {}", args[0]),
        }
    }

    /// 列出文件
    fn cmd_ls(&self, args: &[&str]) {
        println!("Implement ls command");
    }

    /// 打印文件内容
    fn cmd_cat(&self, args: &[&str]) {
        if args.is_empty() {
            println!("Usage: cat <file>");
            return;
        }
        println!("Implement cat command");
    }

    /// 重绘当前行
    fn redraw_line(&self) {
        let mut writer = WRITER.lock();
        writer.set_column(0);
        for _ in 0..BUFFER_WIDTH {
            write!(writer, " ").unwrap();
        }
        writer.set_column(0);
        write!(writer, "{}{}", self.prompt, self.input_buffer).unwrap();
        let prompt_len = self.prompt.len();
        let cursor_column = prompt_len + self.cursor_position;
        writer.set_column(cursor_column);
        write!(writer, " ").unwrap();
        writer.set_column(cursor_column);
    }
}
