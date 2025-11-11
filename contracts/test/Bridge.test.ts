import { expect } from "chai";
import { ethers } from "hardhat";
import { SolanaBridge, WrappedSOL, ValidatorRegistry } from "../typechain-types";
import { SignerWithAddress } from "@nomicfoundation/hardhat-ethers/signers";

describe("SolanaBridge", function () {
  let bridge: SolanaBridge;
  let wrappedSOL: WrappedSOL;
  let validatorRegistry: ValidatorRegistry;
  let owner: SignerWithAddress;
  let user: SignerWithAddress;
  let validator1: SignerWithAddress;
  let validator2: SignerWithAddress;
  let validator3: SignerWithAddress;

  beforeEach(async function () {
    [owner, user, validator1, validator2, validator3] = await ethers.getSigners();

    // Deploy WrappedSOL
    const WrappedSOLFactory = await ethers.getContractFactory("WrappedSOL");
    wrappedSOL = await WrappedSOLFactory.deploy();

    // Deploy ValidatorRegistry (2-of-3 threshold)
    const ValidatorRegistryFactory = await ethers.getContractFactory("ValidatorRegistry");
    validatorRegistry = await ValidatorRegistryFactory.deploy(
      [validator1.address, validator2.address, validator3.address],
      2 // threshold
    );

    // Deploy Bridge
    const BridgeFactory = await ethers.getContractFactory("SolanaBridge");
    bridge = await BridgeFactory.deploy(
      await wrappedSOL.getAddress(),
      await validatorRegistry.getAddress(),
      50 // 0.5% fee
    );

    // Set bridge in WrappedSOL
    await wrappedSOL.setBridge(await bridge.getAddress());
  });

  describe("Deployment", function () {
    it("Should set the correct initial values", async function () {
      expect(await bridge.wrappedSOL()).to.equal(await wrappedSOL.getAddress());
      expect(await bridge.validatorRegistry()).to.equal(await validatorRegistry.getAddress());
      expect(await bridge.feeBasisPoints()).to.equal(50);
      expect(await bridge.owner()).to.equal(owner.address);
    });

    it("Should revert with invalid constructor params", async function () {
      const BridgeFactory = await ethers.getContractFactory("SolanaBridge");

      await expect(
        BridgeFactory.deploy(ethers.ZeroAddress, await validatorRegistry.getAddress(), 50)
      ).to.be.revertedWith("Invalid wSOL address");

      await expect(
        BridgeFactory.deploy(await wrappedSOL.getAddress(), ethers.ZeroAddress, 50)
      ).to.be.revertedWith("Invalid registry address");

      await expect(
        BridgeFactory.deploy(
          await wrappedSOL.getAddress(),
          await validatorRegistry.getAddress(),
          10001
        )
      ).to.be.revertedWith("Fee too high");
    });
  });

  describe("Minting", function () {
    it("Should mint tokens with valid signatures", async function () {
      const amount = ethers.parseEther("10");
      const nonce = 123;

      // Create message
      const message = ethers.solidityPackedKeccak256(
        ["address", "uint256", "uint256", "uint256", "address"],
        [user.address, amount, nonce, 31337, await bridge.getAddress()]
      );

      // Sign with validators
      const sig1 = await validator1.signMessage(ethers.getBytes(message));
      const sig2 = await validator2.signMessage(ethers.getBytes(message));

      // Mint tokens
      await expect(bridge.mintWrapped(user.address, amount, nonce, [sig1, sig2]))
        .to.emit(bridge, "TokensMinted")
        .withArgs(user.address, ethers.parseEther("9.95"), nonce, await ethers.provider.getBlock("latest").then(b => b!.timestamp + 1));

      // Check balance (minus 0.5% fee)
      const expectedAmount = (amount * 9950n) / 10000n;
      expect(await wrappedSOL.balanceOf(user.address)).to.equal(expectedAmount);

      // Check owner received fee
      const expectedFee = (amount * 50n) / 10000n;
      expect(await wrappedSOL.balanceOf(owner.address)).to.equal(expectedFee);
    });

    it("Should prevent replay attacks", async function () {
      const amount = ethers.parseEther("10");
      const nonce = 123;

      const message = ethers.solidityPackedKeccak256(
        ["address", "uint256", "uint256", "uint256", "address"],
        [user.address, amount, nonce, 31337, await bridge.getAddress()]
      );

      const sig1 = await validator1.signMessage(ethers.getBytes(message));
      const sig2 = await validator2.signMessage(ethers.getBytes(message));

      // First mint succeeds
      await bridge.mintWrapped(user.address, amount, nonce, [sig1, sig2]);

      // Second mint with same nonce fails
      await expect(
        bridge.mintWrapped(user.address, amount, nonce, [sig1, sig2])
      ).to.be.revertedWith("Nonce already used");
    });

    it("Should reject insufficient signatures", async function () {
      const amount = ethers.parseEther("10");
      const nonce = 123;
      const message = ethers.solidityPackedKeccak256(
        ["address", "uint256", "uint256", "uint256", "address"],
        [user.address, amount, nonce, 31337, await bridge.getAddress()]
      );

      // Only 1 signature (need 2)
      const sig1 = await validator1.signMessage(ethers.getBytes(message));

      await expect(
        bridge.mintWrapped(user.address, amount, nonce, [sig1])
      ).to.be.revertedWith("Not enough signatures");
    });

    it("Should reject invalid user address", async function () {
      const amount = ethers.parseEther("10");
      const nonce = 123;
      const message = ethers.solidityPackedKeccak256(
        ["address", "uint256", "uint256", "uint256", "address"],
        [ethers.ZeroAddress, amount, nonce, 31337, await bridge.getAddress()]
      );

      const sig1 = await validator1.signMessage(ethers.getBytes(message));
      const sig2 = await validator2.signMessage(ethers.getBytes(message));

      await expect(
        bridge.mintWrapped(ethers.ZeroAddress, amount, nonce, [sig1, sig2])
      ).to.be.revertedWith("Invalid user address");
    });

    it("Should reject zero amount", async function () {
      const amount = 0;
      const nonce = 123;
      const message = ethers.solidityPackedKeccak256(
        ["address", "uint256", "uint256", "uint256", "address"],
        [user.address, amount, nonce, 31337, await bridge.getAddress()]
      );

      const sig1 = await validator1.signMessage(ethers.getBytes(message));
      const sig2 = await validator2.signMessage(ethers.getBytes(message));

      await expect(
        bridge.mintWrapped(user.address, amount, nonce, [sig1, sig2])
      ).to.be.revertedWith("Amount must be > 0");
    });
  });

  describe("Burning", function () {
    beforeEach(async function () {
      // First mint some tokens
      const amount = ethers.parseEther("10");
      const nonce = 123;
      const message = ethers.solidityPackedKeccak256(
        ["address", "uint256", "uint256", "uint256", "address"],
        [user.address, amount, nonce, 31337, await bridge.getAddress()]
      );
      const sig1 = await validator1.signMessage(ethers.getBytes(message));
      const sig2 = await validator2.signMessage(ethers.getBytes(message));
      await bridge.mintWrapped(user.address, amount, nonce, [sig1, sig2]);

      // Approve bridge to burn
      const userBalance = await wrappedSOL.balanceOf(user.address);
      await wrappedSOL.connect(user).approve(await bridge.getAddress(), userBalance);
    });

    it("Should burn tokens and emit event for Solana", async function () {
      const userBalance = await wrappedSOL.balanceOf(user.address);
      const destAddress = "0x1234567890123456789012345678901234567890123456789012345678901234";

      await expect(
        bridge.connect(user).burnAndBridge(userBalance, 0, destAddress)
      ).to.emit(bridge, "TokensBurned");

      // Check balance is zero
      expect(await wrappedSOL.balanceOf(user.address)).to.equal(0);
    });

    it("Should burn tokens and emit event for Sui", async function () {
      const userBalance = await wrappedSOL.balanceOf(user.address);
      const destAddress = "0x1234567890123456789012345678901234567890123456789012345678901234";

      await expect(
        bridge.connect(user).burnAndBridge(userBalance, 2, destAddress)
      ).to.emit(bridge, "TokensBurned");

      expect(await wrappedSOL.balanceOf(user.address)).to.equal(0);
    });

    it("Should reject invalid chain ID", async function () {
      const userBalance = await wrappedSOL.balanceOf(user.address);
      const destAddress = "0x1234567890123456789012345678901234567890123456789012345678901234";

      await expect(
        bridge.connect(user).burnAndBridge(userBalance, 1, destAddress) // 1 is invalid
      ).to.be.revertedWith("Invalid chain (0=Solana, 2=Sui)");
    });

    it("Should reject invalid address length", async function () {
      const userBalance = await wrappedSOL.balanceOf(user.address);
      const destAddress = "0x1234"; // Too short

      await expect(
        bridge.connect(user).burnAndBridge(userBalance, 0, destAddress)
      ).to.be.revertedWith("Invalid address length");
    });

    it("Should reject zero amount", async function () {
      const destAddress = "0x1234567890123456789012345678901234567890123456789012345678901234";

      await expect(
        bridge.connect(user).burnAndBridge(0, 0, destAddress)
      ).to.be.revertedWith("Amount must be > 0");
    });
  });

  describe("Admin Functions", function () {
    it("Should update fee", async function () {
      await expect(bridge.updateFee(100))
        .to.emit(bridge, "FeeUpdated")
        .withArgs(50, 100);

      expect(await bridge.feeBasisPoints()).to.equal(100);
    });

    it("Should reject fee over 100%", async function () {
      await expect(bridge.updateFee(10001)).to.be.revertedWith(
        "Fee too high (max 10000 = 100%)"
      );
    });

    it("Should pause and unpause", async function () {
      await bridge.pause();
      expect(await bridge.paused()).to.be.true;

      // Try minting while paused
      const amount = ethers.parseEther("10");
      const nonce = 456;
      const message = ethers.solidityPackedKeccak256(
        ["address", "uint256", "uint256", "uint256", "address"],
        [user.address, amount, nonce, 31337, await bridge.getAddress()]
      );
      const sig1 = await validator1.signMessage(ethers.getBytes(message));
      const sig2 = await validator2.signMessage(ethers.getBytes(message));

      await expect(
        bridge.mintWrapped(user.address, amount, nonce, [sig1, sig2])
      ).to.be.revertedWithCustomError(bridge, "EnforcedPause");

      // Unpause
      await bridge.unpause();
      expect(await bridge.paused()).to.be.false;

      // Minting should work now
      await bridge.mintWrapped(user.address, amount, nonce, [sig1, sig2]);
      expect(await wrappedSOL.balanceOf(user.address)).to.be.gt(0);
    });

    it("Should update validator registry", async function () {
      const ValidatorRegistryFactory = await ethers.getContractFactory("ValidatorRegistry");
      const newRegistry = await ValidatorRegistryFactory.deploy(
        [validator1.address, validator2.address],
        2
      );

      await expect(bridge.updateValidatorRegistry(await newRegistry.getAddress()))
        .to.emit(bridge, "ValidatorRegistryUpdated");

      expect(await bridge.validatorRegistry()).to.equal(await newRegistry.getAddress());
    });

    it("Should reject non-owner admin calls", async function () {
      await expect(bridge.connect(user).updateFee(100)).to.be.revertedWithCustomError(
        bridge,
        "OwnableUnauthorizedAccount"
      );

      await expect(bridge.connect(user).pause()).to.be.revertedWithCustomError(
        bridge,
        "OwnableUnauthorizedAccount"
      );
    });
  });

  describe("View Functions", function () {
    it("Should check if nonce is processed", async function () {
      const nonce = 123;
      expect(await bridge.isNonceProcessed(nonce)).to.be.false;

      // Mint with this nonce
      const amount = ethers.parseEther("10");
      const message = ethers.solidityPackedKeccak256(
        ["address", "uint256", "uint256", "uint256", "address"],
        [user.address, amount, nonce, 31337, await bridge.getAddress()]
      );
      const sig1 = await validator1.signMessage(ethers.getBytes(message));
      const sig2 = await validator2.signMessage(ethers.getBytes(message));
      await bridge.mintWrapped(user.address, amount, nonce, [sig1, sig2]);

      expect(await bridge.isNonceProcessed(nonce)).to.be.true;
    });

    it("Should get current fee", async function () {
      expect(await bridge.getFee()).to.equal(50);
    });
  });
});
