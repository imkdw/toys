// 창 번호로 화면 위치/크기를 출력한다 (verify.sh 용): x y w h, 그리고 주 화면 크기
import CoreGraphics
import Foundation

let wid = CGWindowID(CommandLine.arguments[1])!
let info = CGWindowListCopyWindowInfo([.optionIncludingWindow], wid) as? [[String: Any]] ?? []
guard let b = info.first?["kCGWindowBounds"] as? [String: CGFloat] else { print("none"); exit(1) }
let main = CGDisplayBounds(CGMainDisplayID())
print(Int(b["X"]!), Int(b["Y"]!), Int(b["Width"]!), Int(b["Height"]!), Int(main.width), Int(main.height))
