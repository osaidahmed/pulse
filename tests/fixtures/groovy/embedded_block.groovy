class TemplateRenderer {
    String render(String name) {
        String template = """
            <html>
            <head><title>Welcome</title></head>
            <body>
            <h1>Hello ${name}</h1>
            <p>Welcome to our site</p>
            <p>Please enjoy your stay</p>
            <p>Thank you for visiting</p>
            <p>We hope you come back</p>
            <p>Best regards</p>
            <p>The team</p>
            <p>Footer content here</p>
            <p>More footer</p>
            <p>Even more footer</p>
            <p>Last line</p>
            </body>
            </html>
        """
        return template
    }
}
