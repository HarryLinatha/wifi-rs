use crate::{
    connectivity::{handlers::NetworkXmlProfileHandler, Connectivity, WifiConnectionError},
    platforms::{Connection, WiFi, WifiError, WifiInterface, AvailableWifi},
};
use std::process::Command;

impl WiFi {
    /// Add the wireless network profile of network to connect to,
    /// (this is specific to windows operating system).
    fn add_profile(ssid: &str, password: &str) -> Result<(), WifiConnectionError> {
        let mut handler = NetworkXmlProfileHandler::new();
        handler.content = handler
            .content
            .replace("{SSID}", ssid)
            .replace("{password}", password);

        let temp_file = handler.write_to_temp_file()?;

        Command::new("netsh")
            .args(&[
                "wlan",
                "add",
                "profile",
                &format!("filename={}", temp_file.path().to_str().unwrap()),
            ])
            .output()
            .map_err(|_| WifiConnectionError::AddNetworkProfileFailed)?;

        Ok(())
    }
}

/// Wireless network connectivity functionality.
impl Connectivity for WiFi {
    /// Attempts to connect to a wireless network with a given SSID and password.
    fn connect(&mut self, ssid: &str, password: &str) -> Result<bool, WifiConnectionError> {
        if !WiFi::is_wifi_enabled().map_err(|err| WifiConnectionError::Other { kind: err })? {
            return Err(WifiConnectionError::Other {
                kind: WifiError::WifiDisabled,
            });
        }

        Self::add_profile(ssid, password)?;

        let output = Command::new("netsh")
            .args(&["wlan", "connect", &format!("name={}", ssid)])
            .output()
            .map_err(|err| WifiConnectionError::FailedToConnect(format!("{}", err)))?;

        if !String::from_utf8_lossy(&output.stdout)
            .as_ref()
            .contains("completed successfully")
        {
            return Ok(false);
        }

        self.connection = Some(Connection {
            ssid: String::from(ssid),
        });

        Ok(true)
    }

    /// Attempts to disconnect from a wireless network currently connected to.
    fn disconnect(&self) -> Result<bool, WifiConnectionError> {
        let output = Command::new("netsh")
            .args(&["wlan", "disconnect"])
            .output()
            .map_err(|err| WifiConnectionError::FailedToDisconnect(format!("{}", err)))?;

        Ok(String::from_utf8_lossy(&output.stdout)
            .as_ref()
            .contains("disconnect"))
    }

    fn scan(&self) -> Result<Vec<AvailableWifi>, WifiError> {
      let mut available_wifis: Vec<AvailableWifi> = Vec::new();

      let output = Command::new("netsh")
        .args(&["wlan", "show", "networks", "mode=bssid"])
        .output()
        .map_err(|err| WifiError::IoError(err))?;

      let output = String::from_utf8_lossy(&output.stdout);
      let lines = output.lines();

      let mut current_ssid = AvailableWifi {
        ssid: String::from(""),
        mac: String::from(""),
        channel: String::from(""),
        signal_level: String::from("0"),
        security: String::from(""),
        in_use: false,
      };
      let mut last_mac = String::from("");
      let mut last_signal_level = String::from("");
      for line in lines {
        if line == "" {
          if current_ssid.mac != "" {
            available_wifis.push(current_ssid.clone());
          }

          current_ssid = AvailableWifi {
            ssid: String::from(""),
            mac: String::from(""),
            channel: String::from(""),
            signal_level: String::from("0"),
            security: String::from(""),
            in_use: false,
          };

          continue;
        }

        let mut parts = line.split_whitespace();
        if parts.clone().count() == 0 { continue; }

        let first = parts.next().unwrap().to_string();
        if first == "Interface" || first == "There" { continue; }

        if first == "SSID" { //SSID 1 : MKM
          parts.next(); //ssid_no
          parts.next(); //colon symbols
          if let Some(temp_ssid) = parts.next() {
            current_ssid.ssid = temp_ssid.to_string();
          }
        } else if first == "Network" { //Network type            : Infrastructure
          continue;
        } else if first == "Authentication" { //Authentication          : WPA2-Personal
          parts.next(); //colon symbols
          if let Some(temp_security) = parts.next() {
            current_ssid.security = temp_security.to_string();
          }
        } else if first == "Encryption" { //Encryption              : CCMP
          continue;
        } else if first == "BSSID" {
          parts.next(); //ssid_no
          parts.next(); //colon symbols
          if let Some(temp_mac) = parts.next() {
            last_mac = temp_mac.to_string();
          } else {
            last_mac = String::from("");
          }
        } else if first == "Signal" { //Signal             : 72%
          parts.next(); //colon symbols
          if let Some(temp_signal) = parts.next() {
            last_signal_level = temp_signal[..temp_signal.len() - 1].to_string();
          } else {
            last_signal_level = String::from("0");
          }
        } else if first == "Radio" { //Radio type         : 802.11ac
          continue;
        } else if first == "Channel" { //Channel            : 10
          parts.next(); //colon symbols
          let mut last_channel = String::from("");
          if let Some(temp_channel) = parts.next() {
            last_channel = temp_channel.to_string();
          }

          let current_signal_level = last_signal_level.parse::<i8>().unwrap();
          let previous_signal_level= current_ssid.signal_level.parse::<i8>().unwrap();

          if current_signal_level > previous_signal_level {
            current_ssid.mac = last_mac.clone();
            current_ssid.signal_level = last_signal_level.clone();
            current_ssid.channel = last_channel;
          }
        } else {
          continue;
        }
      }

      Ok(available_wifis)
    }
}
