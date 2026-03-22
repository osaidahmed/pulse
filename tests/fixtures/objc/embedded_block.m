#import <Foundation/Foundation.h>

@implementation TemplateRenderer

- (NSString *)renderPage:(NSString *)title {
    NSString *html = @"<html>\n"
        "<head>\n"
        "  <meta charset='utf-8'>\n"
        "  <title>Page</title>\n"
        "  <style>\n"
        "    body { margin: 0; padding: 20px; }\n"
        "    .container { max-width: 960px; }\n"
        "    .header { background: #333; color: #fff; }\n"
        "    .nav { display: flex; }\n"
        "    .content { padding: 20px; }\n"
        "    .footer { border-top: 1px solid #ccc; }\n"
        "    .sidebar { width: 200px; }\n"
        "  </style>\n"
        "</head>\n"
        "<body>\n"
        "  <div class='container'></div>\n"
        "</body>\n"
        "</html>";
    return html;
}

@end
