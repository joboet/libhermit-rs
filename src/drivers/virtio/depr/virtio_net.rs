#![allow(dead_code)]

use crate::arch::kernel::pci;
use crate::arch::mm::paging::{BasePageSize, PageSize};
use crate::arch::mm::VirtAddr;
use crate::drivers::virtio::depr::virtio::{
	self, consts::*, virtio_pci_common_cfg, VirtioNotification, Virtq,
};

use alloc::vec::Vec;
use core::sync::atomic::{fence, Ordering};
use core::{fmt, mem, slice, u32, u8};

const VIRTIO_NET_F_CSUM: u64 = 0;
const VIRTIO_NET_F_GUEST_CSUM: u64 = 1;
const VIRTIO_NET_F_CTRL_GUEST_OFFLOADS: u64 = 2;
const VIRTIO_NET_F_MTU: u64 = 3;
const VIRTIO_NET_F_MAC: u64 = 5;
const VIRTIO_NET_F_GUEST_TSO4: u64 = 7;
const VIRTIO_NET_F_GUEST_TSO6: u64 = 8;
const VIRTIO_NET_F_GUEST_ECN: u64 = 9;
const VIRTIO_NET_F_GUEST_UFO: u64 = 10;
const VIRTIO_NET_F_HOST_TSO4: u64 = 11;
const VIRTIO_NET_F_HOST_TSO6: u64 = 12;
const VIRTIO_NET_F_HOST_ECN: u64 = 13;
const VIRTIO_NET_F_HOST_UFO: u64 = 14;
const VIRTIO_NET_F_MRG_RXBUF: u64 = 15;
const VIRTIO_NET_F_STATUS: u64 = 16;
const VIRTIO_NET_F_CTRL_VQ: u64 = 17;
const VIRTIO_NET_F_CTRL_RX: u64 = 18;
const VIRTIO_NET_F_CTRL_VLAN: u64 = 19;
const VIRTIO_NET_F_CTRL_RX_EXTRA: u64 = 20;
const VIRTIO_NET_F_GUEST_ANNOUNCE: u64 = 21;
const VIRTIO_NET_F_MQ: u64 = 22;
const VIRTIO_NET_F_CTRL_MAC_ADDR: u64 = 23;
const VIRTIO_NET_F_RSC_EXT: u64 = 61;
const VIRTIO_NET_F_GSO: u32 = 6;
const VIRTIO_NET_S_LINK_UP: u16 = 1;
const VIRTIO_NET_S_ANNOUNCE: u16 = 2;
/*const VIRTIO_NET_HDR_F_NEEDS_CSUM: u32 = 1;
const VIRTIO_NET_HDR_F_DATA_VALID: u32 = 2;
const VIRTIO_NET_OK: u32 = 0;
const VIRTIO_NET_ERR: u32 = 1;
const VIRTIO_NET_CTRL_RX: u32 = 0;
const VIRTIO_NET_CTRL_RX_PROMISC: u32 = 0;
const VIRTIO_NET_CTRL_RX_ALLMULTI: u32 = 1;
const VIRTIO_NET_CTRL_RX_ALLUNI: u32 = 2;
const VIRTIO_NET_CTRL_RX_NOMULTI: u32 = 3;
const VIRTIO_NET_CTRL_RX_NOUNI: u32 = 4;
const VIRTIO_NET_CTRL_RX_NOBCAST: u32 = 5;
const VIRTIO_NET_CTRL_MAC: u32 = 1;
const VIRTIO_NET_CTRL_MAC_TABLE_SET: u32 = 0;
const VIRTIO_NET_CTRL_MAC_ADDR_SET: u32 = 1;
const VIRTIO_NET_CTRL_VLAN: u32 = 2;
const VIRTIO_NET_CTRL_VLAN_ADD: u32 = 0;
const VIRTIO_NET_CTRL_VLAN_DEL: u32 = 1;
const VIRTIO_NET_CTRL_ANNOUNCE: u32 = 3;
const VIRTIO_NET_CTRL_ANNOUNCE_ACK: u32 = 0;
const VIRTIO_NET_CTRL_MQ: u32 = 4;
const VIRTIO_NET_CTRL_MQ_VQ_PAIRS_SET: u32 = 0;
const VIRTIO_NET_CTRL_MQ_VQ_PAIRS_MIN: u32 = 1;
const VIRTIO_NET_CTRL_MQ_VQ_PAIRS_MAX: u32 = 32768;
const VIRTIO_NET_CTRL_GUEST_OFFLOADS: u32 = 5;
const VIRTIO_NET_CTRL_GUEST_OFFLOADS_SET: u32 = 0;*/

/// use csum_start, csum_offset
const VIRTIO_NET_HDR_F_NEEDS_CSUM: u8 = 1;
/// csum is valid
const VIRTIO_NET_HDR_F_DATA_VALID: u8 = 2;
// reports number of coalesced TCP segments
const VIRTIO_NET_HDR_F_RSC_INFO: u8 = 4;

/// not a GSO frame
const VIRTIO_NET_HDR_GSO_NONE: u8 = 0;
/// GSO frame, IPv4 TCP (TSO)
const VIRTIO_NET_HDR_GSO_TCPV4: u8 = 1;
/// GSO frame, IPv4 UDP (UFO)
const VIRTIO_NET_HDR_GSO_UDP: u8 = 3;
/// GSO frame, IPv6 TCP
const VIRTIO_NET_HDR_GSO_TCPV6: u8 = 4;
/// TCP has ECN set
const VIRTIO_NET_HDR_GSO_ECN: u8 = 0x80;

/// Number of the RX Queue
const VIRTIO_NET_RX_QUEUE: usize = 0;
/// Number of the TX Queue
const VIRTIO_NET_TX_QUEUE: usize = 1;

#[repr(C)]
struct virtio_net_config {
	mac: [u8; 6],
	status: u16,
	max_virtqueue_pairs: u16,
	mtu: u16,
}

impl fmt::Debug for virtio_net_config {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "virtio_net_config {{ ")?;
		write!(
			f,
			"mac: {:x}:{:x}:{:x}:{:x}:{:x}:{:x}, ",
			self.mac[0], self.mac[1], self.mac[2], self.mac[3], self.mac[4], self.mac[5]
		)?;
		write!(f, "max_virtqueue_pairs: {}, ", self.max_virtqueue_pairs)?;
		write!(f, "mtu: {} ", self.mtu)?;
		write!(f, "}}")
	}
}

#[derive(Debug, Default)]
#[repr(C)]
struct virtio_net_hdr_legacy {
	flags: u8,
	gso_type: u8,
	/// Ethernet + IP + tcp/udp hdrs
	hdr_len: u16,
	/// Bytes to append to hdr_len per frame
	gso_size: u16,
	/// Position to start checksumming from
	csum_start: u16,
	/// Offset after that to place checksum
	csum_offset: u16,
	// Number of merged rx buffers
	num_buffers: u16,
}

impl virtio_net_hdr_legacy {
	pub fn init(&mut self, _len: usize) {
		self.flags = 0;
		self.gso_type = VIRTIO_NET_HDR_GSO_NONE;
		self.hdr_len = 0;
		self.gso_size = 0;
		self.csum_start = 0;
		self.csum_offset = 0;
		self.num_buffers = 0;
	}
}

#[derive(Debug, Default)]
#[repr(C)]
struct virtio_net_hdr {
	flags: u8,
	gso_type: u8,
	/// Ethernet + IP + tcp/udp hdrs
	hdr_len: u16,
	/// Bytes to append to hdr_len per frame
	gso_size: u16,
	/// Position to start checksumming from
	csum_start: u16,
	/// Offset after that to place checksum
	csum_offset: u16,
}

impl virtio_net_hdr {
	pub fn init(&mut self, _len: usize) {
		self.flags = 0;
		self.gso_type = VIRTIO_NET_HDR_GSO_NONE;
		self.hdr_len = 0;
		self.gso_size = 0;
		self.csum_start = 0;
		self.csum_offset = 0;
	}
}

#[derive(Debug)]
struct RxBuffer {
	pub addr: VirtAddr,
	pub len: usize,
}

impl RxBuffer {
	pub fn new(len: usize) -> Self {
		let sz = align_up!(len, BasePageSize::SIZE);
		let addr = crate::mm::allocate(sz, true);

		Self { addr, len: sz }
	}
}

impl Drop for RxBuffer {
	fn drop(&mut self) {
		// free buffer
		crate::mm::deallocate(self.addr, self.len);
	}
}

#[derive(Debug)]
struct TxBuffer {
	pub addr: VirtAddr,
	pub len: usize,
	pub in_use: bool,
}

impl TxBuffer {
	pub fn new(len: usize) -> Self {
		let sz = align_up!(len + mem::size_of::<virtio_net_hdr>(), BasePageSize::SIZE);
		let addr = crate::mm::allocate(sz, true);

		Self {
			addr,
			len: sz,
			in_use: false,
		}
	}
}

impl Drop for TxBuffer {
	fn drop(&mut self) {
		// free buffer
		crate::mm::deallocate(self.addr, self.len);
	}
}

pub struct VirtioNetDriver<'a> {
	tx_buffers: Vec<TxBuffer>,
	rx_buffers: Vec<RxBuffer>,
	common_cfg: &'a mut virtio_pci_common_cfg,
	device_cfg: &'a virtio_net_config,
	isr_cfg: &'a mut u32,
	notify_cfg: VirtioNotification,
	vqueues: Option<Vec<Virtq<'a>>>,
}

impl<'a> fmt::Debug for VirtioNetDriver<'a> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "VirtioNetDriver {{ ")?;
		write!(f, "common_cfg: {:?}, ", self.common_cfg)?;
		write!(f, "device_cfg: {:?}, ", self.device_cfg)?;
		write!(f, "isr_cfg: {:#x}, ", self.isr_cfg)?;
		write!(f, "notify_cfg: {:?}, ", self.notify_cfg)?;
		match &self.vqueues {
			None => write!(f, "Uninitialized VQs")?,
			Some(vqs) => write!(f, "Initialized {} VQs", vqs.len())?,
		}
		write!(f, "}}")
	}
}

impl<'a> VirtioNetDriver<'a> {
	pub fn init_vqs(&mut self) {
		let common_cfg = &mut self.common_cfg;
		let notify_cfg = &mut self.notify_cfg;

		debug!("Setting up virtqueues...");

		let vqnum = 2;
		let mut vqueues = Vec::<Virtq<'_>>::new();

		// create the queues and tell device about them
		for i in 0..vqnum as u16 {
			// TODO: catch error
			let vq = Virtq::new_from_common(i, common_cfg, notify_cfg).unwrap();
			vqueues.push(vq);
		}

		let vqsize = common_cfg.queue_size as usize;
		{
			let buffer_size: usize = 65562;
			let vec_buffer = &mut self.rx_buffers;
			for i in 0..vqsize {
				let buffer = RxBuffer::new(buffer_size);
				let addr = buffer.addr;
				vqueues[VIRTIO_NET_RX_QUEUE].add_buffer(i, addr, buffer_size, VIRTQ_DESC_F_WRITE);
				vec_buffer.push(buffer);
			}
		}

		{
			let buffer_size: usize = self.get_mtu() as usize;
			let vec_buffer = &mut self.tx_buffers;
			for i in 0..vqsize {
				let buffer = TxBuffer::new(buffer_size);
				let addr = buffer.addr;
				vqueues[VIRTIO_NET_TX_QUEUE].add_buffer(i, addr, buffer_size, VIRTQ_DESC_F_DEFAULT);
				vec_buffer.push(buffer);
			}
		}

		self.vqueues = Some(vqueues);
	}

	pub fn negotiate_features(&mut self) {
		let common_cfg = &mut self.common_cfg;
		// Linux kernel reads 2x32 featurebits: https://elixir.bootlin.com/linux/latest/ident/vp_get_features
		common_cfg.device_feature_select = 0;
		let mut device_features: u64 = common_cfg.device_feature as u64;
		common_cfg.device_feature_select = 1;
		device_features |= (common_cfg.device_feature as u64) << 32;

		let required: u64 = (1 << VIRTIO_NET_F_MAC)
			| (1 << VIRTIO_NET_F_STATUS)
			| (1 << VIRTIO_NET_F_GUEST_UFO)
			| (1 << VIRTIO_NET_F_GUEST_TSO4)
			| (1 << VIRTIO_NET_F_GUEST_TSO6)
			| (1 << VIRTIO_NET_F_GUEST_CSUM)
			/*| (1 << VIRTIO_NET_F_RSC_EXT) | VIRTIO_F_RING_EVENT_IDX*/;

		if device_features & required == required {
			common_cfg.driver_feature_select = 1;
			common_cfg.driver_feature |= required as u32;
			info!(
				"Virtio features: device {:#x} vs required {:#x}",
				device_features, required
			);
		} else {
			error!("Device doesn't offer required feature to support Virtio-Net");
			error!("Virtio features: {:#x} vs {:#x}", device_features, required);
		}
	}

	pub fn init(&mut self) {
		// 1. Reset the device.
		self.common_cfg.device_status = 0;

		// 2. Set the ACKNOWLEDGE status bit: the guest OS has notice the device.
		self.common_cfg.device_status |= 1;

		// 3. Set the DRIVER status bit: the guest OS knows how to drive the device.
		self.common_cfg.device_status |= 2;

		// 4. Read device feature bits, and write the subset of feature bits understood by the OS and driver to the device.
		//    During this step the driver MAY read (but MUST NOT write) the device-specific configuration fields to check
		//    that it can support the device before accepting it.
		self.negotiate_features();

		// 5. Set the FEATURES_OK status bit. The driver MUST NOT accept new feature bits after this step.
		self.common_cfg.device_status |= 8;

		// 6. Re-read device status to ensure the FEATURES_OK bit is still set:
		//   otherwise, the device does not support our subset of features and the device is unusable.
		if self.common_cfg.device_status & 8 == 0 {
			error!("Device unset FEATURES_OK, aborting!");
			return;
		}

		// 7. Perform device-specific setup, including discovery of virtqueues for the device, optional per-bus setup,
		//    reading and possibly writing the device’s virtio configuration space, and population of virtqueues.
		self.init_vqs();

		// 8. Set the DRIVER_OK status bit. At this point the device is “live”.
		self.common_cfg.device_status |= 4;
	}

	pub fn check_used_elements(&mut self) {
		let buffers = &mut self.tx_buffers;
		while let Some(idx) = (self.vqueues.as_deref_mut().unwrap())[1].check_used_elements() {
			buffers[idx as usize].in_use = false;
		}

		fence(Ordering::SeqCst);
	}

	pub fn handle_interrupt(&mut self) -> bool {
		let isr_status = *(self.isr_cfg);
		if (isr_status & 0x1) == 0x1 {
			return true;
		}

		false
	}

	pub fn set_polling_mode(&mut self, value: bool) {
		(self.vqueues.as_deref_mut().unwrap())[VIRTIO_NET_RX_QUEUE].set_polling_mode(value);
	}

	pub fn get_mac_address(&self) -> [u8; 6] {
		self.device_cfg.mac
	}

	pub fn get_mtu(&self) -> u16 {
		1500 //self.device_cfg.mtu
	}

	pub fn get_tx_buffer(&mut self, len: usize) -> Result<(*mut u8, usize), ()> {
		let buffers = &mut self.tx_buffers;

		// do we have free buffers?
		if !buffers.iter().any(|b| !b.in_use) {
			// if not, check if we are able to free used elements
			self.check_used_elements();
		}

		let index = (self.vqueues.as_ref().unwrap())[VIRTIO_NET_TX_QUEUE].get_available_buffer()?;
		let index = index as usize;

		let buffers = &mut self.tx_buffers;
		if !buffers[index].in_use {
			buffers[index].in_use = true;
			let header = buffers[index].addr.as_mut_ptr::<virtio_net_hdr>();
			unsafe {
				(*header).init(len);
			}

			Ok((
				(buffers[index].addr + mem::size_of::<virtio_net_hdr>()).as_mut_ptr::<u8>(),
				index,
			))
		} else {
			//warn!("Buffer {} is already in use!", index);
			Err(())
		}
	}

	pub fn send_tx_buffer(&mut self, index: usize, len: usize) -> Result<(), ()> {
		(self.vqueues.as_deref_mut().unwrap())[VIRTIO_NET_TX_QUEUE]
			.send_non_blocking(index, len + mem::size_of::<virtio_net_hdr>())
	}

	pub fn has_packet(&self) -> bool {
		(self.vqueues.as_ref().unwrap())[VIRTIO_NET_RX_QUEUE].has_packet()
	}

	pub fn receive_rx_buffer(&self) -> Result<&'static [u8], ()> {
		let (idx, len) = (self.vqueues.as_ref().unwrap())[VIRTIO_NET_RX_QUEUE].get_used_buffer()?;
		let addr = self.rx_buffers[idx as usize].addr;
		let rx_buffer_slice = unsafe {
			slice::from_raw_parts(
				(addr + mem::size_of::<virtio_net_hdr>()).as_ptr::<u8>(),
				len as usize,
			)
		};

		Ok(rx_buffer_slice)
	}

	pub fn rx_buffer_consumed(&mut self) {
		(self.vqueues.as_deref_mut().unwrap())[VIRTIO_NET_RX_QUEUE].buffer_consumed();
	}
}

pub fn create_virtionet_driver(adapter: &pci::PciAdapter) -> Option<VirtioNetDriver<'static>> {
	// Scan capabilities to get common config, which we need to reset the device and get basic info.
	// also see https://elixir.bootlin.com/linux/latest/source/drivers/virtio/virtio_pci_modern.c#L581 (virtio_pci_modern_probe)
	// Read status register
	let bus = adapter.bus;
	let device = adapter.device;
	let status = pci::read_config(bus, device, pci::PCI_COMMAND_REGISTER) >> 16;

	// non-legacy virtio device always specifies capability list, so it can tell us in which bar we find the virtio-config-space
	if status & pci::PCI_STATUS_CAPABILITIES_LIST == 0 {
		error!("Found virtio device without capability list. Likely legacy-device! Aborting.");
		return None;
	}

	// Get pointer to capability list
	let caplist = pci::read_config(bus, device, pci::PCI_CAPABILITY_LIST_REGISTER) & 0xFF;

	// get common config mapped, cast to virtio_pci_common_cfg
	let common_cfg =
		match virtio::map_virtiocap(bus, device, adapter, caplist, VIRTIO_PCI_CAP_COMMON_CFG) {
			Some((cap_common_raw, _)) => unsafe {
				&mut *(cap_common_raw.as_mut_ptr::<virtio_pci_common_cfg>())
			},
			None => {
				error!("Could not find VIRTIO_PCI_CAP_COMMON_CFG. Aborting!");
				return None;
			}
		};
	// get device config mapped, cast to virtio_net_config
	let device_cfg =
		match virtio::map_virtiocap(bus, device, adapter, caplist, VIRTIO_PCI_CAP_DEVICE_CFG) {
			Some((cap_device_raw, _)) => unsafe {
				&mut *(cap_device_raw.as_mut_ptr::<virtio_net_config>())
			},
			None => {
				error!("Could not find VIRTIO_PCI_CAP_DEVICE_CFG. Aborting!");
				return None;
			}
		};
	let isr_cfg = match virtio::map_virtiocap(bus, device, adapter, caplist, VIRTIO_PCI_CAP_ISR_CFG)
	{
		Some((cap_isr_raw, _)) => unsafe { &mut *(cap_isr_raw.as_mut_ptr::<u32>()) },
		None => {
			error!("Could not find VIRTIO_PCI_CAP_ISR_CFG. Aborting!");
			return None;
		}
	};
	// get device notifications mapped
	let (notification_ptr, notify_off_multiplier) =
		match virtio::map_virtiocap(bus, device, adapter, caplist, VIRTIO_PCI_CAP_NOTIFY_CFG) {
			Some((cap_notification_raw, notify_off_multiplier)) => {
				(
					cap_notification_raw.as_mut_ptr::<u16>(), // unsafe { core::slice::from_raw_parts_mut::<u16>(...)}
					notify_off_multiplier,
				)
			}
			None => {
				error!("Could not find VIRTIO_PCI_CAP_NOTIFY_CFG. Aborting!");
				return None;
			}
		};
	let notify_cfg = VirtioNotification {
		notification_ptr,
		notify_off_multiplier,
	};

	// TODO: also load the other cap types (?).

	// Instantiate driver on heap, so it outlives this function
	let mut drv = VirtioNetDriver {
		tx_buffers: Vec::new(),
		rx_buffers: Vec::new(),
		common_cfg,
		device_cfg,
		isr_cfg,
		notify_cfg,
		vqueues: None,
	};

	trace!("Driver before init: {:?}", drv);
	drv.init();
	trace!("Driver after init: {:?}", drv);

	if device_cfg.status & VIRTIO_NET_S_LINK_UP == VIRTIO_NET_S_LINK_UP {
		info!("Virtio-Net link is up");
	} else {
		info!("Virtio-Net link is down");
	}
	info!("Virtio-Net status: {:#x}", drv.common_cfg.device_status);

	Some(drv)
}
