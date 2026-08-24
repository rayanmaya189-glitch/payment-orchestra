package com.paymentorchestra

import com.facebook.react.ReactPackage
import com.facebook.react.bridge.NativeModule
import com.facebook.react.bridge.ReactApplicationContext
import com.facebook.react.uimanager.ViewManager

/**
 * PaymentOrchestraPackage - React Native Android Package
 *
 * Registers native modules for Payment Orchestra.
 */
class PaymentOrchestraPackage : ReactPackage {

    override fun createNativeModules(reactContext: ReactApplicationContext): List<NativeModule> {
        return listOf(
            PaymentOrchestraGooglePayModule(reactContext),
            PaymentOrchestraBridgeModule(reactContext)
        )
    }

    override fun createViewManagers(reactContext: ReactApplicationContext): List<ViewManager<*, *>> {
        return emptyList()
    }
}
