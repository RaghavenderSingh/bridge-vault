import { expect } from "chai";
import { ethers } from "hardhat";
import { ValidatorRegistry } from "../typechain-types";
import { SignerWithAddress } from "@nomicfoundation/hardhat-ethers/signers";

describe("ValidatorRegistry", function () {
  let validatorRegistry: ValidatorRegistry;
  let owner: SignerWithAddress;
  let validator1: SignerWithAddress;
  let validator2: SignerWithAddress;
  let validator3: SignerWithAddress;
  let newValidator: SignerWithAddress;
  let user: SignerWithAddress;

  beforeEach(async function () {
    [owner, validator1, validator2, validator3, newValidator, user] = await ethers.getSigners();

    const ValidatorRegistryFactory = await ethers.getContractFactory("ValidatorRegistry");
    validatorRegistry = await ValidatorRegistryFactory.deploy(
      [validator1.address, validator2.address, validator3.address],
      2 // threshold
    );
  });

  describe("Deployment", function () {
    it("Should set the correct initial values", async function () {
      expect(await validatorRegistry.threshold()).to.equal(2);
      expect(await validatorRegistry.getValidatorCount()).to.equal(3);
      expect(await validatorRegistry.isValidator(validator1.address)).to.be.true;
      expect(await validatorRegistry.isValidator(validator2.address)).to.be.true;
      expect(await validatorRegistry.isValidator(validator3.address)).to.be.true;
    });

    it("Should revert with invalid threshold", async function () {
      const ValidatorRegistryFactory = await ethers.getContractFactory("ValidatorRegistry");

      await expect(
        ValidatorRegistryFactory.deploy([validator1.address, validator2.address], 3)
      ).to.be.revertedWith("Invalid threshold");

      await expect(
        ValidatorRegistryFactory.deploy([validator1.address, validator2.address], 0)
      ).to.be.revertedWith("Threshold must be > 0");
    });

    it("Should revert with duplicate validators", async function () {
      const ValidatorRegistryFactory = await ethers.getContractFactory("ValidatorRegistry");

      await expect(
        ValidatorRegistryFactory.deploy(
          [validator1.address, validator1.address, validator2.address],
          2
        )
      ).to.be.revertedWith("Duplicate validator");
    });

    it("Should revert with zero address validator", async function () {
      const ValidatorRegistryFactory = await ethers.getContractFactory("ValidatorRegistry");

      await expect(
        ValidatorRegistryFactory.deploy([ethers.ZeroAddress, validator1.address], 2)
      ).to.be.revertedWith("Invalid validator");
    });
  });

  describe("Validator Management", function () {
    it("Should add a new validator", async function () {
      await expect(validatorRegistry.addValidator(newValidator.address))
        .to.emit(validatorRegistry, "ValidatorAdded")
        .withArgs(newValidator.address);

      expect(await validatorRegistry.isValidator(newValidator.address)).to.be.true;
      expect(await validatorRegistry.getValidatorCount()).to.equal(4);
    });

    it("Should reject adding existing validator", async function () {
      await expect(validatorRegistry.addValidator(validator1.address)).to.be.revertedWith(
        "Already validator"
      );
    });

    it("Should reject adding zero address", async function () {
      await expect(validatorRegistry.addValidator(ethers.ZeroAddress)).to.be.revertedWith(
        "Invalid address"
      );
    });

    it("Should remove a validator", async function () {
      await expect(validatorRegistry.removeValidator(validator3.address))
        .to.emit(validatorRegistry, "ValidatorRemoved")
        .withArgs(validator3.address);

      expect(await validatorRegistry.isValidator(validator3.address)).to.be.false;
      expect(await validatorRegistry.getValidatorCount()).to.equal(2);
    });

    it("Should reject removing validator if it breaks threshold", async function () {
      // Remove one validator first (3 -> 2)
      await validatorRegistry.removeValidator(validator3.address);

      // Try to remove another (would make it 1, below threshold of 2)
      await expect(validatorRegistry.removeValidator(validator2.address)).to.be.revertedWith(
        "Would break threshold"
      );
    });

    it("Should reject removing non-validator", async function () {
      await expect(validatorRegistry.removeValidator(newValidator.address)).to.be.revertedWith(
        "Not a validator"
      );
    });

    it("Should reject non-owner validator management", async function () {
      await expect(
        validatorRegistry.connect(user).addValidator(newValidator.address)
      ).to.be.revertedWithCustomError(validatorRegistry, "OwnableUnauthorizedAccount");

      await expect(
        validatorRegistry.connect(user).removeValidator(validator1.address)
      ).to.be.revertedWithCustomError(validatorRegistry, "OwnableUnauthorizedAccount");
    });
  });

  describe("Threshold Management", function () {
    it("Should update threshold", async function () {
      await expect(validatorRegistry.updateThreshold(3))
        .to.emit(validatorRegistry, "ThresholdUpdated")
        .withArgs(2, 3);

      expect(await validatorRegistry.threshold()).to.equal(3);
    });

    it("Should reject threshold of zero", async function () {
      await expect(validatorRegistry.updateThreshold(0)).to.be.revertedWith(
        "Threshold must be > 0"
      );
    });

    it("Should reject threshold higher than validator count", async function () {
      await expect(validatorRegistry.updateThreshold(4)).to.be.revertedWith("Threshold too high");
    });

    it("Should reject non-owner threshold update", async function () {
      await expect(
        validatorRegistry.connect(user).updateThreshold(3)
      ).to.be.revertedWithCustomError(validatorRegistry, "OwnableUnauthorizedAccount");
    });
  });

  describe("Signature Verification", function () {
    it("Should verify valid signatures meeting threshold", async function () {
      const message = ethers.solidityPackedKeccak256(
        ["address", "uint256", "uint256"],
        [user.address, ethers.parseEther("10"), 123]
      );

      const sig1 = await validator1.signMessage(ethers.getBytes(message));
      const sig2 = await validator2.signMessage(ethers.getBytes(message));

      const isValid = await validatorRegistry.verifySignatures(message, [sig1, sig2]);
      expect(isValid).to.be.true;
    });

    it("Should verify with more signatures than threshold", async function () {
      const message = ethers.solidityPackedKeccak256(
        ["address", "uint256", "uint256"],
        [user.address, ethers.parseEther("10"), 123]
      );

      const sig1 = await validator1.signMessage(ethers.getBytes(message));
      const sig2 = await validator2.signMessage(ethers.getBytes(message));
      const sig3 = await validator3.signMessage(ethers.getBytes(message));

      const isValid = await validatorRegistry.verifySignatures(message, [sig1, sig2, sig3]);
      expect(isValid).to.be.true;
    });

    it("Should reject insufficient signatures", async function () {
      const message = ethers.solidityPackedKeccak256(
        ["address", "uint256", "uint256"],
        [user.address, ethers.parseEther("10"), 123]
      );

      const sig1 = await validator1.signMessage(ethers.getBytes(message));

      await expect(
        validatorRegistry.verifySignatures(message, [sig1])
      ).to.be.revertedWith("Not enough signatures");
    });

    it("Should reject signatures from non-validators", async function () {
      const message = ethers.solidityPackedKeccak256(
        ["address", "uint256", "uint256"],
        [user.address, ethers.parseEther("10"), 123]
      );

      const sig1 = await validator1.signMessage(ethers.getBytes(message));
      const sig2 = await user.signMessage(ethers.getBytes(message)); // Non-validator

      const isValid = await validatorRegistry.verifySignatures(message, [sig1, sig2]);
      expect(isValid).to.be.false;
    });

    it("Should reject duplicate signatures", async function () {
      const message = ethers.solidityPackedKeccak256(
        ["address", "uint256", "uint256"],
        [user.address, ethers.parseEther("10"), 123]
      );

      const sig1 = await validator1.signMessage(ethers.getBytes(message));

      // Same signature twice
      const isValid = await validatorRegistry.verifySignatures(message, [sig1, sig1]);
      expect(isValid).to.be.false;
    });

    it("Should handle signature verification with removed validator", async function () {
      const message = ethers.solidityPackedKeccak256(
        ["address", "uint256", "uint256"],
        [user.address, ethers.parseEther("10"), 123]
      );

      const sig1 = await validator1.signMessage(ethers.getBytes(message));
      const sig3 = await validator3.signMessage(ethers.getBytes(message));

      // Remove validator3
      await validatorRegistry.removeValidator(validator3.address);

      // Signature from removed validator should be invalid
      const isValid = await validatorRegistry.verifySignatures(message, [sig1, sig3]);
      expect(isValid).to.be.false;
    });
  });

  describe("View Functions", function () {
    it("Should get all validators", async function () {
      const validators = await validatorRegistry.getValidators();
      expect(validators.length).to.equal(3);
      expect(validators).to.include(validator1.address);
      expect(validators).to.include(validator2.address);
      expect(validators).to.include(validator3.address);
    });

    it("Should get validator count", async function () {
      expect(await validatorRegistry.getValidatorCount()).to.equal(3);

      await validatorRegistry.addValidator(newValidator.address);
      expect(await validatorRegistry.getValidatorCount()).to.equal(4);

      await validatorRegistry.removeValidator(newValidator.address);
      expect(await validatorRegistry.getValidatorCount()).to.equal(3);
    });

    it("Should check if address is validator", async function () {
      expect(await validatorRegistry.isValidator(validator1.address)).to.be.true;
      expect(await validatorRegistry.isValidator(user.address)).to.be.false;
    });
  });
});
