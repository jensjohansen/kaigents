# FLM NPU Setup for Ubuntu 25.04 (Plucky)

**Issue:** AMD PPA doesn't support Ubuntu 25.04 yet  
**Solution:** Build from source using AMD's official xdna-driver

---

## 🔧 CORRECTED INSTALLATION COMMANDS

### **Step 1: Clone and Build AMD XDNA Driver**
```bash
# Clone repository with submodules
git clone https://github.com/amd/xdna-driver.git
cd xdna-driver
git submodule update --init --recursive

# Install build dependencies
sudo ./tools/amdxdna_deps.sh

# Build XRT base package
cd xrt/build
./build.sh -npu -opt

# Install base package (adjust version as needed)
sudo apt reinstall ./Release/xrt_202610.2.21.0_25.04-amd64-base.deb
cd ../../
```

### **Step 2: Build XRT Plugin (NPU Support)**
```bash
# Build the NPU plugin
cd build
./build.sh -release

# This creates: build/Release/xrt_plugin.2.21.0_25.04-amd64-amdxdna.deb
```

### **Step 3: Handle Secure Boot (If Enabled)**
```bash
# Import MOK key (you'll be prompted to set a password)
sudo mokutil --import /var/lib/shim-signed/mok/MOK.der

# Reboot to complete enrollment
sudo reboot

# During reboot:
# 1. MOK Manager (blue screen) appears before GRUB
# 2. Select "Enroll MOK" → "Continue" → "Yes"
# 3. Enter the password you set
# 4. Select "Reboot"

# Alternative: Disable Secure Boot in BIOS if you don't want to enroll MOK
```

### **Step 4: Install XRT Plugin**
```bash
# Install the NPU plugin
sudo apt install ./build/Release/xrt_plugin.2.21.0_25.04-amd64-amdxdna.deb
```

### **Step 5: Verify NPU Installation**
```bash
# Load XRT environment
source /opt/xilinx/xrt/setup.sh

# Check device detection
xrt-smi examine

# Run validation tests
xrt-smi validate --device <BDF>

# Expected output:
# - Device detected: NPU Strix at [0000:c2:00.1] (BDF may vary)
# - All validation tests should pass (GEMM, latency, throughput)
```

---

## 🚀 FLM INSTALLATION (After NPU Setup)

### **Step 6: Install FastFlowLM**
```bash
# Download latest FLM release
wget https://github.com/FastFlowLM/FastFlowLM/releases/latest/download/flm_latest_amd64.deb

# Install FLM
sudo apt install ./flm_latest_amd64.deb

# Validate NPU with FLM
flm validate

# Expected output:
# [Linux]  Kernel: 6.14.0-37-generic
# [Linux]  NPU: /dev/accel/accel0
# [Linux]  NPU FW Version: 1.1.0.0+
# [Linux]  Memlock Limit: infinity
```

### **Step 7: Test Hybrid Execution**
```bash
# Set environment variable for NPU beta
export LEMONADE_FLM_LINUX_BETA=1

# Start Lemonade with NPU support
lemonade-server

# Load your Qwen3-Coder-30B model and test hybrid execution
```

---

## 🎯 EXPECTED PERFORMANCE GAINS

**Hybrid NPU+GPU+CPU Execution:**
- **2x tokens/second** improvement
- **10x more power-efficient** 
- **NPU handles prompt processing**
- **GPU handles token generation**
- **CPU handles system operations**

---

## 📊 CURRENT SYSTEM STATUS

**jc01 Specifications:**
- ✅ **CPU:** AMD Ryzen AI 9 HX 370 w/ Radeon 890M
- ✅ **Kernel:** 6.14.0-37-generic (supports amdxdna)
- ✅ **Driver:** amdxdna loaded (version 1.0.0.63)
- ⚠️ **Firmware:** Needs update to 1.1.0.0+
- ✅ **Lemonade:** 9.4.1 installed
- ✅ **Model:** Qwen3-Coder-30B-A3B-Instruct-Q4_K_M.gguf

---

## 🔄 NEXT STEPS

1. **Execute corrected installation** using build-from-source approach
2. **Validate NPU functionality** with `flm validate`
3. **Test hybrid execution** with existing Qwen3-Coder-30B model
4. **Measure performance improvements** vs current GPU+CPU setup
5. **Document results** for competitive advantage analysis

---

## 📞 TROUBLESHOOTING

**Common Issues:**
- **Secure Boot**: Must enroll MOK key or disable Secure Boot
- **Firmware Version**: Update linux-firmware if NPU firmware is outdated
- **Module Loading**: Check `lsmod | grep amdxdna` after installation
- **Device Detection**: Use `xrt-smi examine` to verify NPU detection

**Resources:**
- AMD XDNA Driver: https://github.com/amd/xdna-driver
- FastFlowLM: https://github.com/FastFlowLM/FastFlowLM
- Lemonade NPU Guide: https://lemonade-server.ai/flm_npu_linux.html

---

*This setup enables hybrid NPU+GPU+CPU execution for AMD Ryzen AI processors, giving us a significant performance advantage for AI agent workloads.*
