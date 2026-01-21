using Microsoft.Owin;
using Owin;

[assembly: OwinStartupAttribute(typeof(ContactWeb.Startup))]
namespace ContactWeb
{
    public partial class Startup
    {
        public void Configuration(IAppBuilder app)
        {
            ConfigureAuth(app);
        }
    }
}
