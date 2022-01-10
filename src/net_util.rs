#[path = "../common/config.rs"]
mod config;

use std::{env, process::{Stdio, Command}};
use config::get_config_path;
use sys_util::{geteuid, getegid};
use tun_tap::{Iface, Mode};

fn ip(args: Vec<&str>) -> Result<(), String>{
    let status = Command::new("ip")
    .args(args)
    .stdout(Stdio::inherit()).stderr(Stdio::inherit())
    .status()
    .expect("Failed to start ip");

    if !status.success() {
        return Err(format!("ip failed status: {}", status));
    }
    Ok(())
}

fn iptables(args: Vec<&str>) -> Result<(), String> {
    let status = Command::new("iptables")
    .args(args)
    .stdout(Stdio::inherit()).stderr(Stdio::inherit())
    .status()
    .expect("Failed to start iptables");

    if !status.success() {
        return Err(format!("iptables failed status: {}", status));
    }
    Ok(())
}

fn addtap(tapname: &str) -> Result<(), String> {
    let status = Command::new("tunctl")
    .arg("-t").arg(tapname).arg("-u").arg(env::var("USER").expect("Failed to get USER env var"))
    .stdout(Stdio::inherit()).stderr(Stdio::inherit())
    .status()
    .expect("Failed to start tunctl");

    if !status.success() {
        return Err(format!("tunctl failed status: {}", status));
    }
    println!("TUN device created with name : {}", tapname);
    Ok(())
}

fn addtap_netvm(tapname: &str, brname: &str) -> Result<(), String>{
    // Add the tap device
    addtap(tapname).expect("Failed to create tap device");

    // Add tap device to bridge
    let status = Command::new("brctl")
        .arg("addif").arg(brname).arg(tapname)
        .stdout(Stdio::inherit()).stderr(Stdio::inherit())
        .status()
        .expect("Failed to start net_create");
    if !status.success() {
        return Err(format!("brctl failed status: {}", status));
    }

    // Isolate tap device
    let status = Command::new("bridge")
        .arg("link").arg("set").arg("dev").arg(tapname).arg("isolated").arg("on")
        .stdout(Stdio::inherit()).stderr(Stdio::inherit())
        .status()
        .expect("Failed to start net_create");
    if !status.success() {
        return Err(format!("bridge failed status: {}", status));
    }

    Ok(())
}

fn addtap_host(tapname: &str) -> Result<(), String> {
    // Add the tap device
    addtap(tapname).expect("Failed to create tap device");
    // Set ip address on tap on host
    ip(vec!["addr", "add", "dev", tapname, "10.1.1.1/24"]).unwrap();
    ip(vec!["link", "set", "dev", tapname, "up"]).unwrap();

    let default_iface = env::var("DEFAULT_IFACE").expect("DEFAULT_IFACE needs to be set for host net");
    iptables(vec!["-t", "nat", "-A", "POSTROUTING", "-o", &default_iface, "-j", "MASQUERADE"]).unwrap();
    iptables(vec!["-A", "FORWARD", "-i", &default_iface, "-o", tapname, "-m", "state", "--state", "RELATED,ESTABLISHED", "-j", "ACCEPT"]).unwrap();
    iptables(vec!["-A", "FORWARD", "-o", &default_iface, "-i", tapname, "-j", "ACCEPT"]).unwrap();

    Ok(())
}


fn deltap(tapname: &str) -> Result<(),String>{
    let status = Command::new("ip")
        .arg("link").arg("delete").arg(tapname)
        .stdout(Stdio::inherit()).stderr(Stdio::inherit())
        .status()
        .expect("Failed to start ip");
    
    if !status.success() {
        return Err(format!("ip failed status: {}", status));
    }
    println!("TUN device deleted with name : {}", tapname);
    Ok(())
}

fn main() -> Result<(),String>{
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);
    println!("euid: {}", geteuid());
    println!("egid: {}", getegid());

    match args[1].as_str() {
        "addtap" => {
            match args[2].as_str() {
                "netvm" => {
                    if args.len() != 5 {
                        return Err(format!("Wrong number of arguments : {}", args.len()-1));
                    }
                    return addtap_netvm(&args[3], &args[4]);
                },
                "host" => {
                    if args.len() != 4 {
                        return Err(format!("Wrong number of arguments : {}", args.len()-1));
                    }
                    return addtap_host(&args[3]);
                },
                _ => return Err(format!("network kind not supported: {}", args[2]))
            }
        },
        "deltap" => {
            if args.len() != 3 {
                return Err(format!("Wrong number of arguments : {}", args.len()-1));
            }
            return deltap(&args[2]);
        },
        _ => return Err(format!("Command not supported: {}", args[1]))
    }

    Ok(())
}