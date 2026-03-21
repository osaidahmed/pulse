func buildTemplate() -> String {
    let template = """
    <html>
    <head>
        <title>Page</title>
        <meta charset="utf-8">
        <link rel="stylesheet" href="style.css">
        <script src="app.js"></script>
    </head>
    <body>
        <header>
            <h1>Title</h1>
            <nav>Home | About | Contact</nav>
        </header>
        <main>
            <section>Content goes here</section>
        </main>
        <footer>
            <p>Copyright 2024</p>
        </footer>
    </body>
    </html>
    """
    return template
}
