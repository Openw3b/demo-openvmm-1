use std::{
    path::{PathBuf, Path}, 
    net::Ipv4Addr, str::FromStr,
    fs,
    env
};
use clap::App;
use rand::Rng;
use serde::{Serialize, Deserialize};
use regex::{Regex, Captures};




#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct VmmConfigApp {
    pub kernel_path: String,
    pub vm_init: String,
    pub mount_base: String
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct VmmDependencies {
    pub socat: String,
    pub netutil: String
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct VmmConfig {
    pub dependencies: VmmDependencies,
    pub app: VmmConfigApp
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct VmParentfs {
    pub format: String,
    pub name: String,
    pub version: String,
    pub path: String
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct VmNetworkConfig {
    pub kind: String,
    #[serde(default)]
    pub name: String,
    #[serde(default = "random_tapname")]
    pub tap_ifname: String
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct VmPermission {
    pub kind: String,
    pub value: String
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct  VmDisplayConfig {
    pub main_window: String
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    pub version: String,
    pub kind: String,
    pub name: String,
    #[serde(default = "random_u16")]
    pub vm_cid: u16,

    pub rootfs: VmParentfs,

    pub vcpu_count: u8,
    pub memory: u32,

    pub display: VmDisplayConfig,

    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub mounts: Vec<String>,
    #[serde(default)]
    pub params: Vec<String>,
    #[serde(default)]
    pub networks:  Vec<VmNetworkConfig>,
    #[serde(default)]
    pub permissions: Vec<VmPermission>
}

// Default value generators

fn random_u16() -> u16 {
    let mut rng = rand::thread_rng();
    rng.gen()
}

fn random_tapname() -> String {
    format!("vmtap{}", random_u16())
}


pub fn get_config_path() -> PathBuf{
    if let Ok(val) = env::var("OPENOS_CONFIG_DIR") {
        PathBuf::from(val)
    } else {
        PathBuf::from("./config")
    }
}

pub fn load_vmm_config() -> VmmConfig {
    let mut config_path = get_config_path();
    config_path.push("config.yaml");
    
    let config_str = fs::read_to_string(config_path)
                                .expect("Unable to read config file");
    
    let config = serde_yaml::from_str(&config_str);

    config.unwrap()
}

fn guest_mount_location(path: &PathBuf, vmm_config: &VmmConfig) -> PathBuf {
    let mut ret = PathBuf::from(&vmm_config.app.mount_base);
    ret.push(path.file_name().unwrap());
    ret
}

fn get_slice_limits(expr: &Captures, arg_len: usize) -> (usize, usize) {
    let mut from_arg: usize = expr.get(1)
    .map_or("0", |m| m.as_str())
    .parse().unwrap();
    let mut to_arg: usize = expr.get(2)
        .map_or(
            arg_len.to_string().as_str(), 
            |m| m.as_str())
        .parse().unwrap();
    if from_arg < 0 {
        from_arg += arg_len;
    }
    if to_arg < 0 {
        to_arg += arg_len;
    }

    return (from_arg, to_arg)
}

fn expand_params(params: &Vec<String>, config: &AppConfig, vmm_config: &VmmConfig) -> Vec<String> {

    // TODO: Check host fs security
    let re_args = Regex::new(r"^\$ARGS\[\s*(-?\d+)?\s*:\s*(-?\d+)?\s*\]$").unwrap();
    let re_mounts = Regex::new(r"^\$MOUNTS\[\s*(-?\d+)?\s*:\s*(-?\d+)?\s*\]$").unwrap();

    let arg_len: usize = config.args.len().try_into().unwrap();
    let mounts_len: usize = config.mounts.len().try_into().unwrap();

    let mut updated_params: Vec<String> = Vec::new();
    for param in params {
        let mut replace = false;
        for expr in re_args.captures_iter(&param) {
            let (from_arg, to_arg) = get_slice_limits(&expr, arg_len);
            println!("Replace arg from {:?} to {:?}", from_arg, to_arg);
            replace = true;
            for arg_index in from_arg .. to_arg {
                let path = PathBuf::from(config.args[arg_index].to_string());
                if !path.exists() {
                    panic!("Request mount file {:?} does not exist", path.as_os_str());
                }
                updated_params.push(String::from(path.as_os_str().to_str().unwrap()));
            }
        }
        for expr in re_mounts.captures_iter(&param) {
            let (from_arg, to_arg) = get_slice_limits(&expr, mounts_len);
            println!("Replace mount from {:?} to {:?}", from_arg, to_arg);
            replace = true;
            for param_index in from_arg .. to_arg {
                let path = PathBuf::from(config.mounts[param_index].to_string());
                updated_params.push(String::from(guest_mount_location(&path, &vmm_config).to_str().unwrap()));
            }
        }
        if !replace {
            updated_params.push(param.to_string());
        }
    }

    updated_params
}

pub fn load_app_config(app_name: &str, app_args: &Vec<&str>, vmm_config: &VmmConfig) -> AppConfig {
    let mut config_path = match env::var("OPENOS_CONFIG_DIR") {
        Ok(val) => PathBuf::from(val),
        Err(_) => PathBuf::from("./config"),
    };
    config_path.push("apps");
    config_path.push(String::from(app_name) + ".yaml");

    println!("App Config Path : {:#?}", config_path);
    
    let config_str = fs::read_to_string(config_path)
                                .expect("Unable to read config file");
    let mut config: AppConfig = serde_yaml::from_str(&config_str).unwrap();


    // Add arguments from CLI
    for arg in app_args {
        config.args.push(String::from(*arg));
    }

    config.mounts = expand_params(&config.mounts, &config, &vmm_config);
    config.params = expand_params(&config.params, &config, &vmm_config);

    config
}
