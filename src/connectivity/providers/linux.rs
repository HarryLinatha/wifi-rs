use crate::connectivity::{Connectivity, WifiConnectionError};
use crate::platforms::{Connection, WiFi, WifiError, WifiInterface, AvailableWifi};
use std::process::Command;

/// Wireless network connectivity functionality.
impl Connectivity for WiFi {
    /// Attempts to connect to a wireless network with a given SSID and password.
    fn connect(&mut self, ssid: &str, password: &str) -> Result<bool, WifiConnectionError> {
        if !WiFi::is_wifi_enabled().map_err(|err| WifiConnectionError::Other { kind: err })? {
            return Err(WifiConnectionError::Other {
                kind: WifiError::WifiDisabled,
            });
        }

        let output = Command::new("nmcli")
            .args(&[
                "d",
                "wifi",
                "connect",
                ssid,
                "password",
                &password,
                "ifname",
                &self.interface,
            ])
            .output()
            .map_err(|err| WifiConnectionError::FailedToConnect(format!("{}", err)))?;

        if !String::from_utf8_lossy(&output.stdout)
            .as_ref()
            .contains("successfully activated")
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
        let output = Command::new("nmcli")
            .args(&["d", "disconnect", "ifname", &self.interface])
            .output()
            .map_err(|err| WifiConnectionError::FailedToDisconnect(format!("{}", err)))?;

        Ok(String::from_utf8_lossy(&output.stdout)
            .as_ref()
            .contains("disconnect"))
    }

    // Scan for available networks.
    fn scan(&self) -> Result<Vec<AvailableWifi>, WifiError> {
      let mut available_wifis: Vec<AvailableWifi> = Vec::new();

      let output = Command::new("nmcli")
          .args(&[
            "-f", "IN-USE,BSSID,SSID,CHAN,SIGNAL,SECURITY",
            "d", "wifi", "list"])
          .output()
          .map_err(|err| WifiError::IoError(err))?;

      let output = String::from_utf8_lossy(&output.stdout);
      let mut lines = output.lines();
      lines.next();
      for line in lines {
          let mut parts = line.split_whitespace();
          let temp = parts.next().unwrap().to_string();
          let mut in_use = false;
          let mut mac = String::from("");
          if (temp == "IN-USE") { continue; }
          else if (temp == "*") { in_use = true; mac = parts.next().unwrap().to_string(); }
          else                  { mac = temp; }
          let ssid = parts.next().unwrap().to_string();
          let channel = parts.next().unwrap().to_string();
          let signal_level = parts.next().unwrap().to_string();
          let mut security = parts.next().unwrap().to_string();
          let mut alt_security = String::from("");
          if let Ok(temp) = parts.next() { alt_security = temp.to_string(); }
          if (alt_security != "") { security = alt_security; }
          
          let availableWifi = AvailableWifi {
              ssid,
              mac,
              channel,
              signal_level,
              security,
              in_use,
          };

          available_wifis.push(availableWifi);
      }

      Ok(available_wifis)
  }
}
