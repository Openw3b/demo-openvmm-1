use crate::config::get_config_path;

use super::config::{AppConfig, VmmConfig};
use crosvm::{SharedDirKind, SharedDir};
use devices::{virtio::{GpuParameters, GpuDisplayParameters, GpuMode}, Ac97Parameters};
// use subprocess::{Popen, PopenConfig};
use tun_tap::{Iface, Mode};

use std::{path::PathBuf, fs, process::{Command, Stdio}, env};

pub trait VMM {
    fn load_config(&mut self, config: &AppConfig, temp_dir: &PathBuf) -> Result<(), String>;
}

pub fn run_vm(vmm: &mut VmmCrosVm) -> Result<(), String> {
    if let Err(e) = sys_util::syslog::init() {
        // println!("failed to initialize syslog: {}", e);
        return Err(format!("failed to initialize syslog: {}", e));
    }
    match crosvm::platform::run_config(std::mem::replace(&mut vmm.config, crosvm::Config::default())) {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

// CrosVM
pub struct VmmCrosVm{
    pub config: crosvm::Config,
    tap_names: Vec<String>,
    vmm_config: VmmConfig
}

fn make_gpu_parameters() -> Option<GpuParameters> {
    let mut gpu_params = GpuParameters {
        displays: vec![GpuDisplayParameters {
            width: 1920,
            height: 1080
        }],
        // displays: vec![],
        render_server: None,
        renderer_use_egl: true,
        renderer_use_gles: true,
        renderer_use_glx: false,
        renderer_use_surfaceless: true,
        gfxstream_use_guest_angle: false,
        gfxstream_use_syncfd: true,
        use_vulkan: false,
        mode: GpuMode::Mode2D,
        cache_path: None,
        cache_size: None,
        udmabuf: false,
    };
    
    return Some(gpu_params);
}

struct NetUtil {
    program: String
}
impl NetUtil {
    fn new(program: String) -> NetUtil {
        NetUtil {
            program
        }
    }
    fn run(&self, args: Vec<&str>) -> Result<(), String>{
        let output = Command::new(&self.program)
            .args(args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .expect("Failed to start opennetutil");
        if output.status.success() {
            println!("opennetutil successful");
            return Ok(());
        } else {
            return Err(format!("opennetutil failed with status: {}", output.status));
        }
    }

    fn add_netvm(&self, tap_name: &str, br_name: &str) -> Result<(), String>{
        self.run(vec!["addtap", "netvm", tap_name, br_name])
    }

    fn add_host(&self, tap_name: &str) -> Result<(), String>{
        self.run(vec!["addtap", "host", tap_name])
    }
}


impl VMM for VmmCrosVm {
    fn load_config(&mut self, config: &AppConfig, temp_dir: &PathBuf) -> Result<(), String> {
        self.config = crosvm::Config::default();
        self.config.vcpu_count = Some(config.vcpu_count as usize);
        self.config.memory = Some(config.memory as u64);
        self.config.sandbox = false;
        self.config.cid = Some(config.vm_cid as u64);
        env::set_var("VM_CID", config.vm_cid.to_string());

        // if config.networks.len() > 1 {
        //     return Err(format!("More than 1 networks not supported"));
        // }'
        let netutil = NetUtil::new(self.vmm_config.dependencies.netutil.clone());
        for network in &config.networks {
            match network.kind.as_str() {
                "netvm" => {
                    self.tap_names.push(network.tap_ifname.clone());
                    let br_name = "vmbr_".to_owned() + &network.name;
                    netutil.add_netvm(&network.tap_ifname, &br_name)
                        .expect("Failed to create tap device for netvm");
                    self.config.tap_name.push(network.tap_ifname.clone());
                },
                "host" => {
                    self.tap_names.push(network.tap_ifname.clone());
                    netutil.add_host(&network.tap_ifname)
                        .expect("Failed to create tap device for netvm");
                    self.config.tap_name.push(network.tap_ifname.clone());
                },
                _ => return Err(format!("Unsupported network kink : {}", network.kind))
            }
        }

        // Go over permissions
        for permission in &config.permissions {
            match permission.kind.as_str() {
                "sound" => {
                    // TODO: Fix audio
                    // Set `inside_vm=1` to save some register read ops in the driver.
                    // self.config.params.push("snd_intel8x0.inside_vm=1".to_string());
                    // // Set `ac97_clock=48000` to save intel8x0_measure_ac97_clock call in the driver.
                    // self.config.params.push("snd_intel8x0.ac97_clock=48000".to_string());
                    // let mut ac97_params: Ac97Parameters = Default::default();
                    // ac97_params.capture = true;
                    // self.config.ac97_parameters.push(ac97_params);
                },
                _ => return Err(format!("Invalid permission kind : {}", permission.kind))
            };
        }

        let disk = crosvm::DiskOption {
            path: PathBuf::from(&config.rootfs.path),
            read_only: false,
            o_direct: false,
            sparse: true,
            block_size: 512,
            id: None,
        };
        self.config.params.push("root=/dev/vda rw".to_string());
        self.config.params.push("init=/init".to_string());
        self.config.disks.push(disk);

        for param in &config.params {
            self.config.params.push(param.to_string());
        }


        self.config.gpu_parameters = make_gpu_parameters();
        self.config.software_tpm = false;
        self.config.display_window_keyboard = true;
        self.config.display_window_mouse = true;


        let mut mount_dir = PathBuf::from(temp_dir);
        mount_dir.push("mount");
        fs::create_dir(&mount_dir).expect("Failed to create mount dir");
        fs::write(
            String::from(mount_dir.as_os_str().to_str().unwrap()) + "/app.yaml", 
            serde_yaml::to_string(&config).expect("Failed to serialize ")
        ).expect("Failed to write app.yaml to mounted directory");
        for mount in &config.mounts {
            let mut mount_path = PathBuf::from(temp_dir);
            mount_path.push("mount");
            mount_path.push(PathBuf::from(mount).file_name().unwrap());
            let mount_path_str = mount_path.as_os_str().to_str().unwrap();

            Command::new("touch")
                .arg(mount_path_str)
                .output()
                .expect("failed to execute touch mount_path_str");

            Command::new("mount")
                .args(["--bind", mount.as_str(), mount_path_str])
                .output()
                .expect("failed to execute mount --bind mount mount_path_str");

        }
        let mut shared_dir = SharedDir {
            src: mount_dir.clone(),
            tag: "shared".to_string(),
            kind: SharedDirKind::FS,
            ..Default::default()
        };
        // shared_dir.kind = SharedDirKind::P9;
        self.config.shared_dirs.push(shared_dir);



        // For demo only
        Command::new("sh")
            .arg("-c")
            .arg(format!("/sbin/ip route|awk '/default/ {{ print $3 }}' > {}/host_ip", mount_dir.as_os_str().to_str().unwrap()))
            .output()
            .expect("failed to execute process");

            
        
        arch::set_default_serial_parameters(&mut self.config.serial_parameters);
        
        let kernel_path = PathBuf::from(&self.vmm_config.app.kernel_path);
        if !kernel_path.exists() {
            Err("kernel path does not exist on host".to_string())
        } else {
            self.config.executable_path = Some(crosvm::Executable::Kernel(kernel_path));
            Ok(())
        }
    }
}

impl Drop for VmmCrosVm {
    fn drop(&mut self) {
        println!("Cleaning up tap interfaces");
        for tapname in &self.tap_names {
            let output = Command::new(&self.vmm_config.dependencies.netutil)
                .arg("deltap").arg(&tapname)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .output()
                .expect("Failed to start opennetutil");
            if output.status.success() {
                println!("opennetutil successful");
            } else {
                println!("opennetutil failed with status: {}", output.status);
            }
        }
    }
}

impl VmmCrosVm {
    pub fn new(vmm_config: VmmConfig) -> VmmCrosVm {
        VmmCrosVm {
            config: crosvm::Config::default(),
            tap_names: Default::default(),
            vmm_config
        }
    }
}