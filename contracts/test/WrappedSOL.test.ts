import { expect } from "chai";
import { ethers } from "hardhat";
import { WrappedSOL } from "../typechain-types";
import { SignerWithAddress } from "@nomicfoundation/hardhat-ethers/signers";

describe("WrappedSOL", function () {
  let wrappedSOL: WrappedSOL;
  let owner: SignerWithAddress;
  let bridge: SignerWithAddress;
  let user: SignerWithAddress;

  beforeEach(async function () {
    [owner, bridge, user] = await ethers.getSigners();

    const WrappedSOLFactory = await ethers.getContractFactory("WrappedSOL");
    wrappedSOL = await WrappedSOLFactory.deploy();
  });

  describe("Deployment", function () {
    it("Should set the correct name and symbol", async function () {
      expect(await wrappedSOL.name()).to.equal("Wrapped Solana");
      expect(await wrappedSOL.symbol()).to.equal("wSOL");
    });

    it("Should have 18 decimals", async function () {
      expect(await wrappedSOL.decimals()).to.equal(18);
    });

    it("Should set the owner", async function () {
      expect(await wrappedSOL.owner()).to.equal(owner.address);
    });

    it("Should have zero initial supply", async function () {
      expect(await wrappedSOL.totalSupply()).to.equal(0);
    });

    it("Should not have a bridge set initially", async function () {
      expect(await wrappedSOL.bridge()).to.equal(ethers.ZeroAddress);
    });
  });

  describe("Bridge Management", function () {
    it("Should set bridge address", async function () {
      await expect(wrappedSOL.setBridge(bridge.address))
        .to.emit(wrappedSOL, "BridgeUpdated")
        .withArgs(ethers.ZeroAddress, bridge.address);

      expect(await wrappedSOL.bridge()).to.equal(bridge.address);
    });

    it("Should update bridge address", async function () {
      await wrappedSOL.setBridge(bridge.address);

      await expect(wrappedSOL.setBridge(user.address))
        .to.emit(wrappedSOL, "BridgeUpdated")
        .withArgs(bridge.address, user.address);

      expect(await wrappedSOL.bridge()).to.equal(user.address);
    });

    it("Should reject zero address as bridge", async function () {
      await expect(wrappedSOL.setBridge(ethers.ZeroAddress)).to.be.revertedWith(
        "Invalid bridge address"
      );
    });

    it("Should reject non-owner setting bridge", async function () {
      await expect(
        wrappedSOL.connect(user).setBridge(bridge.address)
      ).to.be.revertedWithCustomError(wrappedSOL, "OwnableUnauthorizedAccount");
    });
  });

  describe("Minting", function () {
    beforeEach(async function () {
      await wrappedSOL.setBridge(bridge.address);
    });

    it("Should mint tokens from bridge", async function () {
      const amount = ethers.parseEther("100");

      await wrappedSOL.connect(bridge).mint(user.address, amount);

      expect(await wrappedSOL.balanceOf(user.address)).to.equal(amount);
      expect(await wrappedSOL.totalSupply()).to.equal(amount);
    });

    it("Should mint multiple times", async function () {
      const amount1 = ethers.parseEther("50");
      const amount2 = ethers.parseEther("75");

      await wrappedSOL.connect(bridge).mint(user.address, amount1);
      await wrappedSOL.connect(bridge).mint(user.address, amount2);

      expect(await wrappedSOL.balanceOf(user.address)).to.equal(amount1 + amount2);
      expect(await wrappedSOL.totalSupply()).to.equal(amount1 + amount2);
    });

    it("Should reject minting to zero address", async function () {
      const amount = ethers.parseEther("100");

      await expect(
        wrappedSOL.connect(bridge).mint(ethers.ZeroAddress, amount)
      ).to.be.revertedWith("Cannot mint to zero address");
    });

    it("Should reject minting from non-bridge", async function () {
      const amount = ethers.parseEther("100");

      await expect(wrappedSOL.connect(user).mint(user.address, amount)).to.be.revertedWith(
        "Only bridge can mint"
      );
    });

    it("Should reject minting before bridge is set", async function () {
      const WrappedSOLFactory = await ethers.getContractFactory("WrappedSOL");
      const newToken = await WrappedSOLFactory.deploy();

      const amount = ethers.parseEther("100");

      await expect(newToken.connect(bridge).mint(user.address, amount)).to.be.revertedWith(
        "Only bridge can mint"
      );
    });
  });

  describe("Burning", function () {
    beforeEach(async function () {
      await wrappedSOL.setBridge(bridge.address);
      // Mint some tokens first
      const amount = ethers.parseEther("100");
      await wrappedSOL.connect(bridge).mint(user.address, amount);
    });

    it("Should burn tokens from bridge", async function () {
      const burnAmount = ethers.parseEther("50");
      const initialBalance = await wrappedSOL.balanceOf(user.address);

      await wrappedSOL.connect(bridge).burnFrom(user.address, burnAmount);

      expect(await wrappedSOL.balanceOf(user.address)).to.equal(initialBalance - burnAmount);
      expect(await wrappedSOL.totalSupply()).to.equal(initialBalance - burnAmount);
    });

    it("Should burn all tokens", async function () {
      const userBalance = await wrappedSOL.balanceOf(user.address);

      await wrappedSOL.connect(bridge).burnFrom(user.address, userBalance);

      expect(await wrappedSOL.balanceOf(user.address)).to.equal(0);
      expect(await wrappedSOL.totalSupply()).to.equal(0);
    });

    it("Should reject burning more than balance", async function () {
      const userBalance = await wrappedSOL.balanceOf(user.address);
      const burnAmount = userBalance + ethers.parseEther("1");

      await expect(
        wrappedSOL.connect(bridge).burnFrom(user.address, burnAmount)
      ).to.be.revertedWithCustomError(wrappedSOL, "ERC20InsufficientBalance");
    });

    it("Should reject burning from non-bridge", async function () {
      const burnAmount = ethers.parseEther("50");

      await expect(
        wrappedSOL.connect(user).burnFrom(user.address, burnAmount)
      ).to.be.revertedWith("Only bridge can burn");
    });
  });

  describe("ERC20 Functionality", function () {
    beforeEach(async function () {
      await wrappedSOL.setBridge(bridge.address);
      // Mint tokens to user
      const amount = ethers.parseEther("100");
      await wrappedSOL.connect(bridge).mint(user.address, amount);
    });

    it("Should transfer tokens", async function () {
      const [, , , recipient] = await ethers.getSigners();
      const transferAmount = ethers.parseEther("25");

      await wrappedSOL.connect(user).transfer(recipient.address, transferAmount);

      expect(await wrappedSOL.balanceOf(user.address)).to.equal(ethers.parseEther("75"));
      expect(await wrappedSOL.balanceOf(recipient.address)).to.equal(transferAmount);
    });

    it("Should approve and transferFrom", async function () {
      const [, , , spender, recipient] = await ethers.getSigners();
      const approveAmount = ethers.parseEther("50");
      const transferAmount = ethers.parseEther("30");

      await wrappedSOL.connect(user).approve(spender.address, approveAmount);
      expect(await wrappedSOL.allowance(user.address, spender.address)).to.equal(approveAmount);

      await wrappedSOL.connect(spender).transferFrom(user.address, recipient.address, transferAmount);

      expect(await wrappedSOL.balanceOf(user.address)).to.equal(ethers.parseEther("70"));
      expect(await wrappedSOL.balanceOf(recipient.address)).to.equal(transferAmount);
      expect(await wrappedSOL.allowance(user.address, spender.address)).to.equal(
        approveAmount - transferAmount
      );
    });

    it("Should reject transfer exceeding balance", async function () {
      const [, , , recipient] = await ethers.getSigners();
      const transferAmount = ethers.parseEther("101");

      await expect(
        wrappedSOL.connect(user).transfer(recipient.address, transferAmount)
      ).to.be.revertedWithCustomError(wrappedSOL, "ERC20InsufficientBalance");
    });

    it("Should reject transferFrom exceeding allowance", async function () {
      const [, , , spender, recipient] = await ethers.getSigners();
      const approveAmount = ethers.parseEther("50");
      const transferAmount = ethers.parseEther("51");

      await wrappedSOL.connect(user).approve(spender.address, approveAmount);

      await expect(
        wrappedSOL.connect(spender).transferFrom(user.address, recipient.address, transferAmount)
      ).to.be.revertedWithCustomError(wrappedSOL, "ERC20InsufficientAllowance");
    });
  });

  describe("Ownership", function () {
    it("Should transfer ownership", async function () {
      const [, newOwner] = await ethers.getSigners();

      await wrappedSOL.transferOwnership(newOwner.address);
      expect(await wrappedSOL.owner()).to.equal(newOwner.address);
    });

    it("Should reject setting bridge from non-owner", async function () {
      const [, newOwner] = await ethers.getSigners();

      await wrappedSOL.transferOwnership(newOwner.address);

      await expect(
        wrappedSOL.connect(owner).setBridge(bridge.address)
      ).to.be.revertedWithCustomError(wrappedSOL, "OwnableUnauthorizedAccount");
    });
  });
});
