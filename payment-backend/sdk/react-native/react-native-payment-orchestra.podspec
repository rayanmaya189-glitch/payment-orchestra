require "json"

package = JSON.parse(File.read(File.join(__dir__, "package.json")))

Pod::Spec.new do |s|
  s.name         = "react-native-payment-orchestra"
  s.version      = package["version"]
  s.summary      = package["description"]
  s.homepage     = package["homepage"]
  s.license      = package["license"]
  s.authors      = package["author"]
  s.source       = { :git => "https://github.com/payment-orchestra/react-native-sdk.git", :tag => s.version }

  s.platform     = :ios, "14.0"
  s.swift_version = "5.0"

  s.source_files = "ios/**/*.{h,m,mm,swift}"

  s.dependency "React-Core"
  s.dependency "PassKit"

  # Google Pay support
  s.frameworks = "PassKit", "WebKit"

  # Use preprocessors for React Native compatibility
  s.compiler_flags = "-DFB_SPECIFIC_SONAME"
end
