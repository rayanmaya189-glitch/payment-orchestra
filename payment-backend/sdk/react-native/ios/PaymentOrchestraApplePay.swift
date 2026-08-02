/**
 * PaymentOrchestraApplePay - iOS Native Module
 *
 * Native bridge for Apple Pay integration in React Native.
 */

import Foundation
import PassKit
import React

@objc(PaymentOrchestraApplePay)
class PaymentOrchestraApplePay: NSObject, RCTBridgeModule, PKPaymentAuthorizationViewControllerDelegate {
    
    static func moduleName() -> String! {
        return "PaymentOrchestraApplePay"
    }
    
    static func requiresMainQueueSetup() -> Bool {
        return true
    }
    
    private var paymentCompletion: RCTPromiseRejectBlock?
    private var paymentResolve: RCTPromiseResolveBlock?
    
    // MARK: - Exported Methods
    
    /**
     * Check if Apple Pay is available
     */
    @objc
    func canMakePayments(
        _ resolve: @escaping RCTPromiseResolveBlock,
        rejecter reject: @escaping RCTPromiseRejectBlock
    ) {
        let canMakePayments = PKPaymentAuthorizationViewController.canMakePayments()
        resolve(NSNumber(value: canMakePayments))
    }
    
    /**
     * Check if a specific card network is available
     */
    @objc
    func canAddCard(
        _ cardNetwork: String,
        resolver resolve: @escaping RCTPromiseResolveBlock,
        rejecter reject: @escaping RCTPromiseRejectBlock
    ) {
        let network = mapCardNetwork(cardNetwork)
        
        if #available(iOS 13.0, *) {
            PKPaymentAuthorizationController.canMakePayments(usingNetworks: [network]) { canMake in
                resolve(NSNumber(value: canMake))
            }
        } else {
            let canMake = PKPaymentAuthorizationViewController.canMakePayments(usingNetworks: [network])
            resolve(NSNumber(value: canMake))
        }
    }
    
    /**
     * Authorize Apple Pay payment
     */
    @objc
    func authorize(
        _ options: NSDictionary,
        resolver resolve: @escaping RCTPromiseResolveBlock,
        rejecter reject: @escaping RCTPromiseRejectBlock
    ) {
        guard let merchantIdentifier = options["merchantIdentifier"] as? String else {
            reject("INVALID_REQUEST", "Merchant identifier is required", nil)
            return
        }
        
        let countryCode = options["countryCode"] as? String ?? "US"
        let currencyCode = options["currencyCode"] as? String ?? "USD"
        let supportedNetworks = options["supportedNetworks"] as? [String] ?? ["visa", "mastercard", "amex"]
        let merchantCapabilities = options["merchantCapabilities"] as? [String] ?? ["3ds"]
        let amount = options["amount"] as? NSNumber
        let label = options["label"] as? String ?? "Payment"
        
        // Create payment request
        let paymentRequest = PKPaymentRequest()
        paymentRequest.merchantIdentifier = merchantIdentifier
        paymentRequest.countryCode = countryCode
        paymentRequest.currencyCode = currencyCode
        paymentRequest.supportedNetworks = supportedNetworks.map { mapCardNetwork($0) }
        paymentRequest.merchantCapabilities = mapMerchantCapabilities(merchantCapabilities)
        
        // Set payment summary items
        var summaryItems: [PKPaymentSummaryItem] = []
        
        if let amount = amount {
            let paymentAmount = NSDecimalNumber(decimal: amount.decimalValue)
            let summaryItem = PKPaymentSummaryItem(
                label: label,
                amount: paymentAmount,
                type: .final
            )
            summaryItems.append(summaryItem)
        }
        
        paymentRequest.paymentSummaryItems = summaryItems
        
        // Present Apple Pay
        DispatchQueue.main.async { [weak self] in
            guard let self = self else { return }
            
            guard let viewController = RCTPresentedViewController() else {
                reject("NO_VIEW_CONTROLLER", "No view controller available", nil)
                return
            }
            
            if #available(iOS 14.0, *) {
                let controller = PKPaymentAuthorizationController(paymentRequest: paymentRequest)
                controller.delegate = self
                self.paymentResolve = resolve
                self.paymentCompletion = reject
                
                controller.present { presented in
                    if !presented {
                        reject("PRESENTATION_FAILED", "Failed to present Apple Pay", nil)
                    }
                }
            } else {
                guard let viewController = PKPaymentAuthorizationViewController(paymentRequest: paymentRequest) as PKPaymentAuthorizationViewController? else {
                    reject("INITIALIZATION_FAILED", "Failed to initialize Apple Pay", nil)
                    return
                }
                
                viewController.delegate = self
                self.paymentResolve = resolve
                self.paymentCompletion = reject
                
                viewController.modalPresentationStyle = .formSheet
                viewController.modalTransitionStyle = .coverVertical
                viewController.navigationItem.leftBarButtonItem = UIBarButtonItem(
                    barButtonSystemItem: .cancel,
                    target: self,
                    action: #selector(self.cancelPayment)
                )
                
                viewController.present(viewController, animated: true, completion: nil)
            }
        }
    }
    
    /**
     * Add card to Apple Pay
     */
    @objc
    func addCard(
        _ options: NSDictionary,
        resolver resolve: @escaping RCTPromiseResolveBlock,
        rejecter reject: @escaping RCTPromiseRejectBlock
    ) {
        guard let merchantIdentifier = options["merchantIdentifier"] as? String else {
            reject("INVALID_REQUEST", "Merchant identifier is required", nil)
            return
        }
        
        let countryCode = options["countryCode"] as? String ?? "US"
        let supportedNetworks = options["supportedNetworks"] as? [String] ?? ["visa", "mastercard"]
        let merchantCapabilities = options["merchantCapabilities"] as? [String] ?? ["3ds"]
        
        // Create pass library for adding card
        let passLibrary = PKPassLibrary()
        
        // Check if we can add to Apple Pay
        guard PKPaymentAuthorizationViewController.canMakePayments() else {
            reject("NOT_AVAILABLE", "Apple Pay is not available on this device", nil)
            return
        }
        
        // Note: Adding cards programmatically requires a payment provider integration
        // This is a simplified implementation
        reject("NOT_IMPLEMENTED", "Adding cards requires payment provider integration", nil)
    }
    
    // MARK: - PKPaymentAuthorizationViewControllerDelegate
    
    func paymentAuthorizationViewController(
        _ controller: PKPaymentAuthorizationViewController,
        didAuthorizePayment payment: PKPayment,
        handler completion: @escaping (PKPaymentAuthorizationResult) -> Void
    ) {
        // Extract payment token
        guard let token = payment.token else {
            completion(PKPaymentAuthorizationResult(status: .failure, errors: nil))
            return
        }
        
        // Create token data
        let paymentData = token.paymentData
        let transactionIdentifier = token.transactionIdentifier
        
        let result: [String: Any] = [
            "paymentData": paymentData.base64EncodedString(),
            "transactionIdentifier": transactionIdentifier,
            "paymentMethod": [
                "displayName": token.paymentMethod.displayName ?? "",
                "network": token.paymentMethod.network?.rawValue ?? "",
                "type": mapPaymentMethodType(token.paymentMethod.type)
            ]
        ]
        
        DispatchQueue.main.async { [weak self] in
            self?.paymentResolve?(result)
            self?.paymentResolve = nil
            self?.paymentCompletion = nil
        }
        
        completion(PKPaymentAuthorizationResult(status: .success, errors: nil))
    }
    
    func paymentAuthorizationViewControllerDidFinish(
        _ controller: PKPaymentAuthorizationViewController
    ) {
        controller.dismiss(animated: true)
        
        // If completion hasn't been called yet, user canceled
        if let reject = paymentCompletion {
            reject("CANCELED", "Payment was canceled", nil)
            paymentCompletion = nil
            paymentResolve = nil
        }
    }
    
    @objc func cancelPayment() {
        if let reject = paymentCompletion {
            reject("CANCELED", "Payment was canceled", nil)
            paymentCompletion = nil
            paymentResolve = nil
        }
    }
    
    // MARK: - Helper Methods
    
    private func mapCardNetwork(_ network: String) -> PKPaymentNetwork {
        switch network.lowercased() {
        case "visa":
            return .visa
        case "mastercard":
            return .masterCard
        case "amex":
            return .amex
        case "discover":
            return .discover
        case "jcb":
            if #available(iOS 12.1.1, *) {
                return .jcb
            }
            return .visa
        case "interac":
            return .interac
        case "cartebancaire":
            if #available(iOS 12.0, *) {
                return .cartesBancaires
            }
            return .visa
        default:
            return .visa
        }
    }
    
    private func mapMerchantCapabilities(_ capabilities: [String]) -> PKMerchantCapability {
        var result: PKMerchantCapability = []
        
        for capability in capabilities {
            switch capability.lowercased() {
            case "3ds":
                result.insert(.capability3DS)
            case "emv":
                result.insert(.capabilityEMV)
            case "credit":
                result.insert(.capabilityCredit)
            case "debit":
                result.insert(.capabilityDebit)
            default:
                break
            }
        }
        
        if result.isEmpty {
            result = .capability3DS
        }
        
        return result
    }
    
    private func mapPaymentMethodType(_ type: PKPaymentMethodType) -> String {
        switch type {
        case .debit:
            return "debit"
        case .credit:
            return "credit"
        case .prepaid:
            return "prepaid"
        case .store:
            return "store"
        case .eMoney:
            return "emoney"
        default:
            return "unknown"
        }
    }
}

// MARK: - React Native Bridge Support

@objc(PaymentOrchestraApplePayModule)
class PaymentOrchestraApplePayModule: NSObject {
    
    @objc static func requiresMainQueueSetup() -> Bool {
        return false
    }
}
