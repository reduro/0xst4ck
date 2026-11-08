using System;
using System.IO;
using System.IO.Compression;
using System.Reflection;
using System.Diagnostics;
using System.Drawing;
using System.Text;
using System.Windows.Forms;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;

[assembly: AssemblyTitle("Assistant d'installation de Clemtout Launcher")]
[assembly: AssemblyDescription("Installation officielle de Clemtout Launcher")]
[assembly: AssemblyCompany("Clemtout")]
[assembly: AssemblyProduct("Clemtout Launcher")]
[assembly: AssemblyCopyright("Copyright (C) 2026")]
[assembly: AssemblyVersion("2.1.0.0")]
[assembly: AssemblyFileVersion("2.1.0.0")]

namespace ClemtoutInstaller
{
    // Native Shell COM Interface for real .lnk shortcuts
    [ComImport]
    [Guid("00021401-0000-0000-C000-000000000046")]
    internal class ShellLink { }

    [ComImport]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    [Guid("000214F9-0000-0000-C000-000000000046")]
    internal interface IShellLinkW
    {
        void GetPath([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszFile, int cchMaxPath, out IntPtr pfd, int fFlags);
        void GetIDList(out IntPtr ppidl);
        void SetIDList(IntPtr pidl);
        void GetDescription([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszName, int cchMaxName);
        void SetDescription([MarshalAs(UnmanagedType.LPWStr)] string pszName);
        void GetWorkingDirectory([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszDir, int cchMaxPath);
        void SetWorkingDirectory([MarshalAs(UnmanagedType.LPWStr)] string pszDir);
        void GetArguments([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszArgs, int cchMaxPath);
        void SetArguments([MarshalAs(UnmanagedType.LPWStr)] string pszArgs);
        void GetHotkey(out short pwHotkey);
        void SetHotkey(short wHotkey);
        void GetShowCmd(out int piShowCmd);
        void SetShowCmd(int iShowCmd);
        void GetIconLocation([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszIconPath, int cchIconPath, out int piIcon);
        void SetIconLocation([MarshalAs(UnmanagedType.LPWStr)] string pszIconPath, int iIcon);
        void SetRelativePath([MarshalAs(UnmanagedType.LPWStr)] string pszPathRel, int dwReserved);
        void Resolve(IntPtr hwnd, int fFlags);
        void SetPath([MarshalAs(UnmanagedType.LPWStr)] string pszFile);
    }

    public class StandardSetupForm : Form
    {
        // Panels for wizard pages
        private Panel headerPanel;
        private Label headerTitle;
        private Label headerSubtitle;
        private PictureBox headerIcon;

        private Panel footerPanel;
        private Button btnCancel;
        private Button btnNext;

        // Page 1: Destination & Shortcuts
        private Panel page1;
        private TextBox txtPath;
        private CheckBox chkDesktop;
        private CheckBox chkStartMenu;

        // Page 2: Progress
        private Panel page2;
        private ProgressBar prgInstall;
        private Label lblInstallStatus;

        // Page 3: Complete
        private Panel page3;
        private CheckBox chkLaunchNow;

        private int currentPage = 1;
        private string targetDirectory = "";

        public StandardSetupForm()
        {
            this.Text = "Installation - Clemtout Launcher";
            this.Size = new Size(560, 420);
            this.StartPosition = FormStartPosition.CenterScreen;
            this.FormBorderStyle = FormBorderStyle.FixedSingle;
            this.MaximizeBox = false;
            this.Font = new Font("Segoe UI", 9f);
            this.Icon = Icon.ExtractAssociatedIcon(Application.ExecutablePath);

            // Default install directory
            targetDirectory = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                "Programs",
                "ClemtoutLauncher"
            );

            BuildHeader();
            BuildFooter();
            BuildPage1();
            BuildPage2();
            BuildPage3();

            ShowPage(1);
        }

        private void BuildHeader()
        {
            headerPanel = new Panel();
            headerPanel.Dock = DockStyle.Top;
            headerPanel.Height = 72;
            headerPanel.BackColor = Color.White;
            headerPanel.Paint += (s, e) =>
            {
                using (Pen p = new Pen(Color.FromArgb(215, 215, 215)))
                {
                    e.Graphics.DrawLine(p, 0, headerPanel.Height - 1, headerPanel.Width, headerPanel.Height - 1);
                }
            };

            headerTitle = new Label();
            headerTitle.Text = "Sélection du dossier de destination";
            headerTitle.Font = new Font("Segoe UI", 10.5f, FontStyle.Bold);
            headerTitle.Location = new Point(24, 14);
            headerTitle.Size = new Size(420, 22);
            headerTitle.ForeColor = Color.FromArgb(30, 30, 30);
            headerPanel.Controls.Add(headerTitle);

            headerSubtitle = new Label();
            headerSubtitle.Text = "Où Clemtout Launcher doit-il être installé ?";
            headerSubtitle.Font = new Font("Segoe UI", 8.5f);
            headerSubtitle.Location = new Point(36, 38);
            headerSubtitle.Size = new Size(420, 20);
            headerSubtitle.ForeColor = Color.FromArgb(90, 90, 90);
            headerPanel.Controls.Add(headerSubtitle);

            headerIcon = new PictureBox();
            headerIcon.Location = new Point(486, 12);
            headerIcon.Size = new Size(48, 48);
            headerIcon.SizeMode = PictureBoxSizeMode.Zoom;
            try { headerIcon.Image = Icon.ExtractAssociatedIcon(Application.ExecutablePath).ToBitmap(); } catch { }
            headerPanel.Controls.Add(headerIcon);

            this.Controls.Add(headerPanel);
        }

        private void BuildFooter()
        {
            footerPanel = new Panel();
            footerPanel.Dock = DockStyle.Bottom;
            footerPanel.Height = 56;
            footerPanel.BackColor = SystemColors.Control;
            footerPanel.Paint += (s, e) =>
            {
                using (Pen p = new Pen(Color.FromArgb(215, 215, 215)))
                {
                    e.Graphics.DrawLine(p, 0, 0, footerPanel.Width, 0);
                }
            };

            btnCancel = new Button();
            btnCancel.Text = "Annuler";
            btnCancel.Location = new Point(444, 14);
            btnCancel.Size = new Size(88, 28);
            btnCancel.Click += (s, e) => this.Close();
            footerPanel.Controls.Add(btnCancel);

            btnNext = new Button();
            btnNext.Text = "Installer >";
            btnNext.Location = new Point(348, 14);
            btnNext.Size = new Size(88, 28);
            btnNext.Click += (s, e) => OnNextClick();
            footerPanel.Controls.Add(btnNext);

            this.Controls.Add(footerPanel);
        }

        private void BuildPage1()
        {
            page1 = new Panel();
            page1.Location = new Point(0, 72);
            page1.Size = new Size(544, 250);

            Label lblIntro = new Label();
            lblIntro.Text = "Le programme d'installation va installer Clemtout Launcher dans le dossier suivant.\nPour continuer, cliquez sur Installer. Si vous souhaitez choisir un autre dossier, cliquez sur Parcourir.";
            lblIntro.Location = new Point(28, 18);
            lblIntro.Size = new Size(490, 40);
            page1.Controls.Add(lblIntro);

            GroupBox grpPath = new GroupBox();
            grpPath.Text = "Dossier d'installation";
            grpPath.Location = new Point(28, 68);
            grpPath.Size = new Size(490, 68);

            txtPath = new TextBox();
            txtPath.Text = targetDirectory;
            txtPath.Location = new Point(14, 26);
            txtPath.Size = new Size(365, 25);
            grpPath.Controls.Add(txtPath);

            Button btnBrowse = new Button();
            btnBrowse.Text = "Parcourir...";
            btnBrowse.Location = new Point(386, 24);
            btnBrowse.Size = new Size(88, 28);
            btnBrowse.Click += (s, e) =>
            {
                using (FolderBrowserDialog fbd = new FolderBrowserDialog())
                {
                    fbd.SelectedPath = txtPath.Text;
                    if (fbd.ShowDialog() == DialogResult.OK)
                    {
                        txtPath.Text = fbd.SelectedPath;
                    }
                }
            };
            grpPath.Controls.Add(btnBrowse);
            page1.Controls.Add(grpPath);

            GroupBox grpOpts = new GroupBox();
            grpOpts.Text = "Options supplémentaires";
            grpOpts.Location = new Point(28, 148);
            grpOpts.Size = new Size(490, 80);

            chkDesktop = new CheckBox();
            chkDesktop.Text = "Créer un raccourci sur le Bureau";
            chkDesktop.Checked = true;
            chkDesktop.Location = new Point(16, 22);
            chkDesktop.Size = new Size(400, 24);
            grpOpts.Controls.Add(chkDesktop);

            chkStartMenu = new CheckBox();
            chkStartMenu.Text = "Créer un raccourci dans le Menu Démarrer";
            chkStartMenu.Checked = true;
            chkStartMenu.Location = new Point(16, 48);
            chkStartMenu.Size = new Size(400, 24);
            grpOpts.Controls.Add(chkStartMenu);

            page1.Controls.Add(grpOpts);
            this.Controls.Add(page1);
        }

        private void BuildPage2()
        {
            page2 = new Panel();
            page2.Location = new Point(0, 72);
            page2.Size = new Size(544, 250);

            Label lblWait = new Label();
            lblWait.Text = "Veuillez patienter pendant l'installation de Clemtout Launcher sur votre ordinateur...";
            lblWait.Location = new Point(28, 28);
            lblWait.Size = new Size(490, 25);
            page2.Controls.Add(lblWait);

            prgInstall = new ProgressBar();
            prgInstall.Location = new Point(28, 70);
            prgInstall.Size = new Size(490, 24);
            prgInstall.Style = ProgressBarStyle.Continuous;
            page2.Controls.Add(prgInstall);

            lblInstallStatus = new Label();
            lblInstallStatus.Text = "Préparation de l'installation...";
            lblInstallStatus.Location = new Point(28, 106);
            lblInstallStatus.Size = new Size(490, 24);
            lblInstallStatus.ForeColor = Color.FromArgb(70, 70, 70);
            page2.Controls.Add(lblInstallStatus);

            this.Controls.Add(page2);
        }

        private void BuildPage3()
        {
            page3 = new Panel();
            page3.Location = new Point(0, 72);
            page3.Size = new Size(544, 250);

            Label lblDoneTitle = new Label();
            lblDoneTitle.Text = "Installation terminée";
            lblDoneTitle.Font = new Font("Segoe UI", 12f, FontStyle.Bold);
            lblDoneTitle.Location = new Point(28, 25);
            lblDoneTitle.Size = new Size(490, 30);
            page3.Controls.Add(lblDoneTitle);

            Label lblDoneDesc = new Label();
            lblDoneDesc.Text = "Clemtout Launcher a été installé avec succès sur votre ordinateur.\n\nCliquez sur Terminer pour quitter l'assistant.";
            lblDoneDesc.Location = new Point(28, 65);
            lblDoneDesc.Size = new Size(490, 48);
            page3.Controls.Add(lblDoneDesc);

            chkLaunchNow = new CheckBox();
            chkLaunchNow.Text = "Lancer Clemtout Launcher maintenant";
            chkLaunchNow.Checked = true;
            chkLaunchNow.Location = new Point(28, 135);
            chkLaunchNow.Size = new Size(350, 26);
            page3.Controls.Add(chkLaunchNow);

            this.Controls.Add(page3);
        }

        private void ShowPage(int page)
        {
            currentPage = page;
            page1.Visible = (page == 1);
            page2.Visible = (page == 2);
            page3.Visible = (page == 3);

            if (page == 1)
            {
                headerTitle.Text = "Sélection du dossier de destination";
                headerSubtitle.Text = "Où Clemtout Launcher doit-il être installé ?";
                btnNext.Text = "Installer";
                btnNext.Enabled = true;
                btnCancel.Enabled = true;
            }
            else if (page == 2)
            {
                headerTitle.Text = "Installation en cours";
                headerSubtitle.Text = "Copie des fichiers nécessaires au bon fonctionnement...";
                btnNext.Enabled = false;
                btnCancel.Enabled = false;
            }
            else if (page == 3)
            {
                headerTitle.Text = "Fin de l'installation";
                headerSubtitle.Text = "L'application est prête à être utilisée.";
                btnNext.Text = "Terminer";
                btnNext.Enabled = true;
                btnCancel.Visible = false;
            }
        }

        private void OnNextClick()
        {
            if (currentPage == 1)
            {
                targetDirectory = txtPath.Text.Trim();
                ShowPage(2);
                Application.DoEvents();
                ExecuteInstallation();
            }
            else if (currentPage == 3)
            {
                if (chkLaunchNow.Checked)
                {
                    string exeTarget = Path.Combine(targetDirectory, "Clemtout-Launcher.exe");
                    if (File.Exists(exeTarget))
                    {
                        Process.Start(new ProcessStartInfo(exeTarget) { WorkingDirectory = targetDirectory });
                    }
                }
                this.Close();
            }
        }

        private void ExecuteInstallation()
        {
            try
            {
                prgInstall.Value = 10;
                lblInstallStatus.Text = "Vérification des processus actifs...";
                Application.DoEvents();

                // Safe check: Close only running Clemtout Launcher instances
                foreach (var proc in Process.GetProcessesByName("Clemtout-Launcher"))
                {
                    try { proc.Kill(); proc.WaitForExit(1500); } catch { }
                }
                foreach (var proc in Process.GetProcessesByName("clemtout-launcher"))
                {
                    try { proc.Kill(); proc.WaitForExit(1500); } catch { }
                }

                if (!Directory.Exists(targetDirectory))
                {
                    Directory.CreateDirectory(targetDirectory);
                }

                prgInstall.Value = 35;
                lblInstallStatus.Text = "Extraction des fichiers de l'application...";
                Application.DoEvents();

                Assembly asm = Assembly.GetExecutingAssembly();
                using (Stream stream = asm.GetManifestResourceStream("payload.zip"))
                {
                    if (stream == null) throw new Exception("Le composant d'installation 'payload.zip' est manquant.");
                    using (ZipArchive archive = new ZipArchive(stream))
                    {
                        int total = archive.Entries.Count;
                        int current = 0;
                        foreach (ZipArchiveEntry entry in archive.Entries)
                        {
                            current++;
                            string destPath = Path.Combine(targetDirectory, entry.FullName);
                            string destDir = Path.GetDirectoryName(destPath);
                            if (!Directory.Exists(destDir)) Directory.CreateDirectory(destDir);

                            entry.ExtractToFile(destPath, true);

                            int pct = 35 + (int)((current / (double)total) * 45);
                            prgInstall.Value = Math.Min(80, pct);
                            lblInstallStatus.Text = "Extraction : " + entry.Name;
                            Application.DoEvents();
                        }
                    }
                }

                prgInstall.Value = 85;
                lblInstallStatus.Text = "Configuration des raccourcis Windows...";
                Application.DoEvents();

                string exePath = Path.Combine(targetDirectory, "Clemtout-Launcher.exe");
                string icoPath = Path.Combine(targetDirectory, "icon.ico");

                if (chkDesktop.Checked)
                {
                    string desktopLnk = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory), "Clemtout Launcher.lnk");
                    CreateNativeShortcut(desktopLnk, exePath, targetDirectory, icoPath, "Lanceur de jeux et réseau multijoueur");
                }

                if (chkStartMenu.Checked)
                {
                    string startMenu = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.StartMenu), "Programs");
                    string menuLnk = Path.Combine(startMenu, "Clemtout Launcher.lnk");
                    CreateNativeShortcut(menuLnk, exePath, targetDirectory, icoPath, "Lanceur de jeux et réseau multijoueur");
                }

                prgInstall.Value = 100;
                lblInstallStatus.Text = "Installation terminée avec succès !";
                Application.DoEvents();

                System.Threading.Thread.Sleep(300);
                ShowPage(3);
            }
            catch (Exception ex)
            {
                MessageBox.Show(
                    "Une erreur est survenue pendant l'installation :\n\n" + ex.Message,
                    "Erreur d'installation",
                    MessageBoxButtons.OK,
                    MessageBoxIcon.Error
                );
                ShowPage(1);
            }
        }

        private static void CreateNativeShortcut(string shortcutPath, string targetPath, string workingDir, string iconLocation, string description)
        {
            try
            {
                IShellLinkW link = (IShellLinkW)new ShellLink();
                link.SetPath(targetPath);
                link.SetWorkingDirectory(workingDir);
                link.SetDescription(description);

                if (File.Exists(iconLocation))
                {
                    link.SetIconLocation(iconLocation, 0);
                }

                IPersistFile file = (IPersistFile)link;
                file.Save(shortcutPath, false);
            }
            catch { }
        }

        [STAThread]
        public static void Main()
        {
            Application.EnableVisualStyles();
            Application.SetCompatibleTextRenderingDefault(false);
            Application.Run(new StandardSetupForm());
        }
    }
}
