// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

/**
 * @title ValidatorRegistry
 * @notice Manages validator addresses and verifies multi-signature authentication
 * @dev Uses ECDSA for signature verification with configurable threshold
 */
contract ValidatorRegistry is Ownable {
    using ECDSA for bytes32;
    using MessageHashUtils for bytes32;

    mapping(address => bool) public isValidator;
    address[] public validators;
    uint256 public threshold;

    event ValidatorAdded(address indexed validator);
    event ValidatorRemoved(address indexed validator);
    event ThresholdUpdated(uint256 oldThreshold, uint256 newThreshold);

    /**
     * @notice Constructor initializes validators and threshold
     * @param _validators Array of initial validator addresses
     * @param _threshold Minimum number of signatures required
     */
    constructor(address[] memory _validators, uint256 _threshold) Ownable(msg.sender) {
        require(_validators.length >= _threshold, "Invalid threshold");
        require(_threshold > 0, "Threshold must be > 0");

        for (uint256 i = 0; i < _validators.length; i++) {
            require(_validators[i] != address(0), "Invalid validator");
            require(!isValidator[_validators[i]], "Duplicate validator");

            isValidator[_validators[i]] = true;
            validators.push(_validators[i]);
        }

        threshold = _threshold;
    }

    /**
     * @notice Add a new validator
     * @param validator Address of the validator to add
     * @dev Only owner can add validators
     */
    function addValidator(address validator) external onlyOwner {
        require(!isValidator[validator], "Already validator");
        require(validator != address(0), "Invalid address");

        isValidator[validator] = true;
        validators.push(validator);

        emit ValidatorAdded(validator);
    }

    /**
     * @notice Remove a validator
     * @param validator Address of the validator to remove
     * @dev Only owner can remove validators. Cannot remove if it breaks threshold
     */
    function removeValidator(address validator) external onlyOwner {
        require(isValidator[validator], "Not a validator");
        require(validators.length - 1 >= threshold, "Would break threshold");

        isValidator[validator] = false;

        // Remove from array
        for (uint256 i = 0; i < validators.length; i++) {
            if (validators[i] == validator) {
                validators[i] = validators[validators.length - 1];
                validators.pop();
                break;
            }
        }

        emit ValidatorRemoved(validator);
    }

    /**
     * @notice Update the signature threshold
     * @param _threshold New threshold value
     * @dev Only owner can update threshold
     */
    function updateThreshold(uint256 _threshold) external onlyOwner {
        require(_threshold > 0, "Threshold must be > 0");
        require(_threshold <= validators.length, "Threshold too high");

        uint256 oldThreshold = threshold;
        threshold = _threshold;
        emit ThresholdUpdated(oldThreshold, _threshold);
    }

    /**
     * @notice Verify that signatures meet the threshold requirement
     * @param messageHash Hash of the message that was signed
     * @param signatures Array of signatures to verify
     * @return bool True if signatures are valid and meet threshold
     */
    function verifySignatures(
        bytes32 messageHash,
        bytes[] calldata signatures
    ) external view returns (bool) {
        require(signatures.length >= threshold, "Not enough signatures");

        bytes32 ethSignedHash = messageHash.toEthSignedMessageHash();
        address[] memory signers = new address[](signatures.length);
        uint256 validCount = 0;

        for (uint256 i = 0; i < signatures.length; i++) {
            address signer = ethSignedHash.recover(signatures[i]);

            // Check if valid validator and not duplicate
            if (isValidator[signer] && !_contains(signers, signer, validCount)) {
                signers[validCount] = signer;
                validCount++;
            }
        }

        return validCount >= threshold;
    }

    /**
     * @notice Get the list of all validator addresses
     * @return address[] Array of validator addresses
     */
    function getValidators() external view returns (address[] memory) {
        return validators;
    }

    /**
     * @notice Get the count of validators
     * @return uint256 Number of validators
     */
    function getValidatorCount() external view returns (uint256) {
        return validators.length;
    }

    /**
     * @notice Check if an address array contains a specific address
     * @param arr Array to search
     * @param addr Address to find
     * @param len Length to search (optimization)
     * @return bool True if address is found
     */
    function _contains(address[] memory arr, address addr, uint256 len)
        private pure returns (bool)
    {
        for (uint256 i = 0; i < len; i++) {
            if (arr[i] == addr) return true;
        }
        return false;
    }
}
