// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

contract ValidatorRegistry is Ownable {
    using ECDSA for bytes32;
    using MessageHashUtils for bytes32;

    mapping(address => bool) public isValidator;
    address[] public validators;
    uint256 public threshold;

    event ValidatorAdded(address indexed validator);
    event ValidatorRemoved(address indexed validator);
    event ThresholdUpdated(uint256 oldThreshold, uint256 newThreshold);

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

    function addValidator(address validator) external onlyOwner {
        require(!isValidator[validator], "Already validator");
        require(validator != address(0), "Invalid address");

        isValidator[validator] = true;
        validators.push(validator);

        emit ValidatorAdded(validator);
    }

    function removeValidator(address validator) external onlyOwner {
        require(isValidator[validator], "Not a validator");
        require(validators.length - 1 >= threshold, "Would break threshold");

        isValidator[validator] = false;

        for (uint256 i = 0; i < validators.length; i++) {
            if (validators[i] == validator) {
                validators[i] = validators[validators.length - 1];
                validators.pop();
                break;
            }
        }

        emit ValidatorRemoved(validator);
    }

    function updateThreshold(uint256 _threshold) external onlyOwner {
        require(_threshold > 0, "Threshold must be > 0");
        require(_threshold <= validators.length, "Threshold too high");

        uint256 oldThreshold = threshold;
        threshold = _threshold;
        emit ThresholdUpdated(oldThreshold, _threshold);
    }

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

            if (isValidator[signer] && !_contains(signers, signer, validCount)) {
                signers[validCount] = signer;
                validCount++;
            }
        }

        return validCount >= threshold;
    }

    function getValidators() external view returns (address[] memory) {
        return validators;
    }

    function getValidatorCount() external view returns (uint256) {
        return validators.length;
    }

    function _contains(address[] memory arr, address addr, uint256 len)
        private pure returns (bool)
    {
        for (uint256 i = 0; i < len; i++) {
            if (arr[i] == addr) return true;
        }
        return false;
    }
}
